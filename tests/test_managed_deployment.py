import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]


class ManagedDeploymentTests(unittest.TestCase):
    def test_systemd_request_contract_uses_one_private_marker(self):
        request_path = "/var/lib/pontemesh-server/home/state/update-request.json"
        path_unit = (ROOT / "deploy/systemd/pontemesh-update.path").read_text()
        server_drop_in = (
            ROOT / "deploy/systemd/pontemesh-server.service.d/10-panel-update.conf"
        ).read_text()
        update_drop_in = (
            ROOT / "deploy/systemd/pontemesh-update.service.d/10-panel-request.conf"
        ).read_text()

        self.assertIn(f"PathExists={request_path}", path_unit)
        self.assertIn(f"PONTEMESH_UPDATE_REQUEST_FILE={request_path}", server_drop_in)
        self.assertIn(f"ExecStartPre=/usr/bin/rm -f {request_path}", update_drop_in)
        self.assertIn("Unit=pontemesh-update.service", path_unit)

    def test_nginx_fallback_stays_independent_from_the_application(self):
        config = (ROOT / "deploy/nginx/pontemesh-web-fallback.conf").read_text()
        page = (ROOT / "deploy/nginx/pontemesh-unavailable.html").read_text()

        self.assertIn("proxy_intercept_errors on;", config)
        self.assertIn("error_page 502 503 504 =503 /pontemesh-unavailable.html;", config)
        self.assertIn("internal;", config)
        self.assertIn("root /var/www/pontemesh;", config)
        self.assertIn("systemctl status pontemesh-server", page)
        self.assertIn("journalctl -u pontemesh-server", page)


if __name__ == "__main__":
    unittest.main()
