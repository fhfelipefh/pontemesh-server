#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PYTHONPYCACHEPREFIX="${PYTHONPYCACHEPREFIX:-/tmp/pontemesh-sdk-update-tests-pycache}" \
  python3 -m unittest tests.test_github_release_update
