#!/usr/bin/env python3
import argparse
import fnmatch
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request


SEMVER = re.compile(r"^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$")


def fail(message, code=1):
    print(json.dumps({"ok": False, "error": message}, indent=2, sort_keys=True))
    raise SystemExit(code)


def parse_version(value):
    match = SEMVER.match(value.strip())
    if not match:
        raise ValueError(f"invalid semver: {value}")
    major, minor, patch, prerelease = match.groups()
    return (int(major), int(minor), int(patch), prerelease or "")


def is_newer(remote, current):
    remote_v = parse_version(remote)
    current_v = parse_version(current)
    if remote_v[:3] != current_v[:3]:
        return remote_v[:3] > current_v[:3]
    if remote_v[3] == current_v[3]:
        return False
    if not remote_v[3] and current_v[3]:
        return True
    if remote_v[3] and not current_v[3]:
        return False
    return remote_v[3] > current_v[3]


def read_current_version(root):
    cargo = root / "Cargo.toml"
    pattern = re.compile(r'(?m)^version\s*=\s*"([^"]+)"')
    match = pattern.search(cargo.read_text())
    if not match:
        fail(f"could not read version from {cargo}")
    return match.group(1)


def api_request(url, token=None):
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "pontemesh-release-updater",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.loads(response.read().decode("utf-8"))


def download(url, path, token=None):
    headers = {"User-Agent": "pontemesh-release-updater"}
    if "api.github.com" in url:
        headers["Accept"] = "application/octet-stream"
        if token:
            headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=120) as response:
        with path.open("wb") as output:
            shutil.copyfileobj(response, output)


def sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def ensure_trusted_repository(state_dir, repository, trust_on_first_use):
    trust_file = state_dir / "trusted-repository"
    if trust_file.exists():
        trusted = trust_file.read_text().strip()
        if trusted != repository:
            fail(f"trusted repository mismatch: expected {trusted}, got {repository}")
        return trusted
    if not trust_on_first_use:
        fail("repository is not trusted yet; rerun with --trust-on-first-use after verifying it")
    trust_file.write_text(repository + "\n")
    return repository


def interval_allows_check(state_file, interval_seconds, force):
    if force or not state_file.exists():
        return True
    try:
        state = json.loads(state_file.read_text())
    except json.JSONDecodeError:
        return True
    checked_at = float(state.get("checkedAtEpoch", 0))
    return time.time() - checked_at >= interval_seconds


def release_candidates(repository, include_prerelease, token):
    releases = api_request(f"https://api.github.com/repos/{repository}/releases", token)
    candidates = []
    for release in releases:
        if release.get("draft"):
            continue
        if release.get("prerelease") and not include_prerelease:
            continue
        tag = str(release.get("tag_name", ""))
        try:
            parse_version(tag)
        except ValueError:
            continue
        candidates.append(release)
    return sorted(candidates, key=lambda release: parse_version(str(release["tag_name"])), reverse=True)


def find_asset(release, pattern):
    assets = release.get("assets") or []
    for asset in assets:
        if fnmatch.fnmatch(asset.get("name", ""), pattern):
            return asset
    return None


def load_manifest(release, product, version, stage_dir, token):
    pattern = f"pontemesh-{product}-v{version}-manifest.json"
    asset = find_asset(release, pattern)
    if not asset:
        fail(f"release manifest asset not found: {pattern}")
    manifest_path = stage_dir / pattern
    download(asset["url"], manifest_path, token)
    manifest = json.loads(manifest_path.read_text())
    if manifest.get("product") != product:
        fail("release manifest product mismatch")
    if manifest.get("version") != version:
        fail("release manifest version mismatch")
    return manifest, manifest_path


def select_manifest_asset(manifest, pattern):
    assets = manifest.get("assets")
    if not isinstance(assets, list) or not assets:
        fail("release manifest has no assets")
    for asset in assets:
        name = str(asset.get("name", ""))
        if fnmatch.fnmatch(name, pattern):
            return asset
    fail(f"no manifest asset matched {pattern}")


def stage_asset(release, manifest, asset_pattern, stage_dir, token):
    wanted = select_manifest_asset(manifest, asset_pattern)
    asset = find_asset(release, wanted["name"])
    if not asset:
        fail(f"github release asset not found: {wanted['name']}")
    staged_path = stage_dir / wanted["name"]
    download(asset["url"], staged_path, token)
    size = staged_path.stat().st_size
    digest = sha256_file(staged_path)
    if int(wanted["size"]) != size:
        fail(f"downloaded asset size mismatch for {wanted['name']}")
    if str(wanted["sha256"]).lower() != digest:
        fail(f"downloaded asset sha256 mismatch for {wanted['name']}")
    return {
        "name": wanted["name"],
        "path": str(staged_path),
        "size": size,
        "sha256": digest,
    }


def spawn_background(args, root, state_dir):
    log_dir = state_dir / "logs"
    log_dir.mkdir(parents=True, exist_ok=True)
    log_path = log_dir / "update-check.log"
    next_args = [sys.executable, str(pathlib.Path(__file__).resolve())]
    next_args += [item for item in args if item != "--background"]
    env = dict(os.environ)
    env["PONTEMESH_UPDATE_BACKGROUND"] = "1"
    with log_path.open("ab") as log:
        subprocess.Popen(next_args, cwd=root, env=env, stdout=log, stderr=log, start_new_session=True)
    return {"ok": True, "background": True, "log": str(log_path)}


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--product", required=True, choices=["sdk", "server"])
    parser.add_argument("--repository", default=os.environ.get("PONTEMESH_UPDATE_REPOSITORY"))
    parser.add_argument("--current-version")
    parser.add_argument("--root", default=".")
    parser.add_argument("--state-dir", default=os.environ.get("PONTEMESH_UPDATE_STATE_DIR", "target/update-state"))
    parser.add_argument("--stage-dir", default=os.environ.get("PONTEMESH_UPDATE_STAGE_DIR", "target/update-staging"))
    parser.add_argument("--interval-seconds", type=int, default=24 * 60 * 60)
    parser.add_argument("--asset-pattern", default="*")
    parser.add_argument("--include-prerelease", action="store_true")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--stage", action="store_true")
    parser.add_argument("--background", action="store_true")
    parser.add_argument("--trust-on-first-use", action="store_true")
    args = parser.parse_args()

    root = pathlib.Path(args.root).resolve()
    state_dir = pathlib.Path(args.state_dir).resolve()
    stage_dir = pathlib.Path(args.stage_dir).resolve()
    state_dir.mkdir(parents=True, exist_ok=True)
    if args.stage:
        stage_dir.mkdir(parents=True, exist_ok=True)

    if args.background and os.environ.get("PONTEMESH_UPDATE_BACKGROUND") != "1":
        print(json.dumps(spawn_background(sys.argv[1:], root, state_dir), indent=2, sort_keys=True))
        return

    if not args.repository:
        fail("repository is required as --repository or PONTEMESH_UPDATE_REPOSITORY")
    if not re.match(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$", args.repository):
        fail("repository must use owner/name")

    ensure_trusted_repository(state_dir, args.repository, args.trust_on_first_use)
    current_version = args.current_version or read_current_version(root)
    state_file = state_dir / f"{args.product}-last-check.json"
    report_file = state_dir / f"{args.product}-update-report.json"

    if not interval_allows_check(state_file, args.interval_seconds, args.force):
        state = json.loads(state_file.read_text())
        state["ok"] = True
        state["skipped"] = "interval-not-elapsed"
        print(json.dumps(state, indent=2, sort_keys=True))
        return

    token = os.environ.get("GITHUB_TOKEN")
    try:
        releases = release_candidates(args.repository, args.include_prerelease, token)
    except urllib.error.HTTPError as error:
        fail(f"github api request failed: {error.code}")
    except urllib.error.URLError as error:
        fail(f"github api request failed: {error.reason}")

    newer = []
    for release in releases:
        tag = str(release["tag_name"])
        if is_newer(tag, current_version):
            newer.append(release)

    result = {
        "ok": True,
        "checkedAtEpoch": int(time.time()),
        "product": args.product,
        "repository": args.repository,
        "currentVersion": current_version,
        "updateAvailable": bool(newer),
    }

    if newer:
        release = newer[0]
        version = str(release["tag_name"]).lstrip("v")
        result.update(
            {
                "latestVersion": version,
                "releaseUrl": release.get("html_url"),
                "staged": False,
            }
        )
        if args.stage:
            manifest, manifest_path = load_manifest(release, args.product, version, stage_dir, token)
            staged = stage_asset(release, manifest, args.asset_pattern, stage_dir, token)
            result["staged"] = True
            result["manifestPath"] = str(manifest_path)
            result["asset"] = staged

    write_json(state_file, result)
    write_json(report_file, result)
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
