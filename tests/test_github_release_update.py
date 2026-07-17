import contextlib
import hashlib
import http.server
import io
import importlib.util
import json
import pathlib
import socketserver
import subprocess
import tempfile
import threading
import time
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "github-release-update.py"


def load_updater():
    spec = importlib.util.spec_from_file_location("github_release_update", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        return


@contextlib.contextmanager
def local_http_root(root):
    handler = lambda *args, **kwargs: QuietHandler(*args, directory=str(root), **kwargs)
    server = socketserver.TCPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_address[1]}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


class GithubReleaseUpdateTests(unittest.TestCase):
    def setUp(self):
        self.updater = load_updater()

    def test_semver_comparison(self):
        self.assertTrue(self.updater.is_newer("v0.2.0", "0.1.9"))
        self.assertTrue(self.updater.is_newer("v1.0.0", "1.0.0-rc.1"))
        self.assertFalse(self.updater.is_newer("v1.0.0-rc.1", "1.0.0"))
        self.assertFalse(self.updater.is_newer("v0.1.0", "0.1.0"))
        with self.assertRaises(ValueError):
            self.updater.parse_version("latest")

    def test_repository_trust_is_pinned(self):
        with tempfile.TemporaryDirectory() as temp:
            state = pathlib.Path(temp)
            self.assertEqual(
                self.updater.ensure_trusted_repository(state, "owner/project", True),
                "owner/project",
            )
            self.assertEqual(
                (state / "trusted-repository").read_text().strip(),
                "owner/project",
            )
            with contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaises(SystemExit):
                    self.updater.ensure_trusted_repository(state, "other/project", True)

    def test_interval_skip_does_not_create_stage_dir_or_call_network(self):
        with tempfile.TemporaryDirectory() as temp:
            temp_path = pathlib.Path(temp)
            state = temp_path / "state"
            stage = temp_path / "stage"
            state.mkdir()
            (state / "trusted-repository").write_text("owner/project\n")
            (state / "sdk-last-check.json").write_text(
                json.dumps(
                    {
                        "checkedAtEpoch": int(time.time()),
                        "product": "sdk",
                        "repository": "owner/project",
                        "currentVersion": "0.1.0",
                        "updateAvailable": False,
                    }
                )
            )
            result = subprocess.run(
                [
                    str(SCRIPT),
                    "--product",
                    "sdk",
                    "--repository",
                    "owner/project",
                    "--current-version",
                    "0.1.0",
                    "--state-dir",
                    str(state),
                    "--stage-dir",
                    str(stage),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=True,
            )
            payload = json.loads(result.stdout)
            self.assertEqual(payload["skipped"], "interval-not-elapsed")
            self.assertFalse(stage.exists())

    def test_manifest_and_asset_are_staged_from_local_http(self):
        with tempfile.TemporaryDirectory() as temp:
            temp_path = pathlib.Path(temp)
            http_root = temp_path / "http"
            stage = temp_path / "stage"
            http_root.mkdir()
            stage.mkdir()
            asset_name = "pontemesh-sdk-v0.2.0-linux-x64.tar.gz"
            asset_bytes = b"release-asset-bytes"
            (http_root / asset_name).write_bytes(asset_bytes)
            digest = hashlib.sha256(asset_bytes).hexdigest()
            manifest = {
                "schema": 1,
                "product": "sdk",
                "version": "0.2.0",
                "assets": [
                    {
                        "name": asset_name,
                        "size": len(asset_bytes),
                        "sha256": digest,
                    }
                ],
            }
            (http_root / "manifest.json").write_text(json.dumps(manifest))
            with local_http_root(http_root) as base_url:
                release = {
                    "tag_name": "v0.2.0",
                    "assets": [
                        {
                            "name": "pontemesh-sdk-v0.2.0-manifest.json",
                            "url": f"{base_url}/manifest.json",
                        },
                        {
                            "name": asset_name,
                            "url": f"{base_url}/{asset_name}",
                        },
                    ],
                }
                loaded_manifest, manifest_path = self.updater.load_manifest(
                    release, "sdk", "0.2.0", stage, None
                )
                staged = self.updater.stage_asset(
                    release, loaded_manifest, "*linux-x64.tar.gz", stage, None
                )
                self.assertEqual(manifest_path.read_text(), json.dumps(manifest))
                self.assertEqual(staged["sha256"], digest)
                self.assertEqual((stage / asset_name).read_bytes(), asset_bytes)

    def test_bad_asset_digest_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            temp_path = pathlib.Path(temp)
            http_root = temp_path / "http"
            stage = temp_path / "stage"
            http_root.mkdir()
            stage.mkdir()
            asset_name = "pontemesh-server-v0.2.0-windows-x64.zip"
            (http_root / asset_name).write_bytes(b"actual-content")
            manifest = {
                "schema": 1,
                "product": "server",
                "version": "0.2.0",
                "assets": [
                    {
                        "name": asset_name,
                        "size": len(b"actual-content"),
                        "sha256": "0" * 64,
                    }
                ],
            }
            with local_http_root(http_root) as base_url:
                release = {
                    "tag_name": "v0.2.0",
                    "assets": [
                        {
                            "name": asset_name,
                            "url": f"{base_url}/{asset_name}",
                        }
                    ],
                }
                with contextlib.redirect_stdout(io.StringIO()):
                    with self.assertRaises(SystemExit):
                        self.updater.stage_asset(release, manifest, "*.zip", stage, None)


if __name__ == "__main__":
    unittest.main()
