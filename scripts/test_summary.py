#!/usr/bin/env python3
"""
Generates a visual ASCII/Unicode box summary of test results and code coverage
for GitHub Actions step summary ($GITHUB_STEP_SUMMARY) and console log.
"""
import json
import os
import re
import sys

ANSI_ESCAPE = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')

def clean_text(text):
    return ANSI_ESCAPE.sub('', text)

def parse_cargo_test_output(log_path):
    passed = 0
    failed = 0
    ignored = 0
    if not os.path.exists(log_path):
        return passed, failed, ignored

    with open(log_path, "r", encoding="utf-8", errors="ignore") as f:
        content = clean_text(f.read())

    matches = re.findall(r"test result: (?:ok|FAILED)\.\s+(\d+)\s+passed;\s+(\d+)\s+failed;\s+(\d+)\s+ignored", content)
    if matches:
        p, f, i = matches[-1]
        passed, failed, ignored = int(p), int(f), int(i)

    return passed, failed, ignored

def parse_vitest_output(log_path):
    passed = 0
    failed = 0
    ignored = 0
    if not os.path.exists(log_path):
        return passed, failed, ignored

    with open(log_path, "r", encoding="utf-8", errors="ignore") as f:
        content = clean_text(f.read())

    m_passed = re.search(r"Tests\s+.*?\b(\d+)\s+passed", content)
    if m_passed:
        passed = int(m_passed.group(1))

    m_failed = re.search(r"Tests\s+.*?\b(\d+)\s+failed", content)
    if m_failed:
        failed = int(m_failed.group(1))

    m_skipped = re.search(r"Tests\s+.*?\b(\d+)\s+skipped", content)
    if m_skipped:
        ignored = int(m_skipped.group(1))

    return passed, failed, ignored

def parse_unittest_output(log_path):
    passed = 0
    failed = 0
    ignored = 0
    if not os.path.exists(log_path):
        return passed, failed, ignored

    with open(log_path, "r", encoding="utf-8", errors="ignore") as f:
        content = clean_text(f.read())

    m_ran = re.search(r"Ran\s+(\d+)\s+test", content)
    if m_ran:
        total = int(m_ran.group(1))
        m_failures = re.search(r"FAILED\s*\((?:failures=(\d+))?(?:,\s*)?(?:errors=(\d+))?\)", content)
        if m_failures:
            f1 = int(m_failures.group(1) or 0)
            f2 = int(m_failures.group(2) or 0)
            failed = f1 + f2
            passed = max(0, total - failed)
        else:
            passed = total

    return passed, failed, ignored

def format_summary():
    cargo_log = sys.argv[1] if len(sys.argv) > 1 else "cargo-test.log"
    vitest_log = sys.argv[2] if len(sys.argv) > 2 else "vitest.log"
    python_log = sys.argv[3] if len(sys.argv) > 3 else "unittest.log"
    cov_json = sys.argv[4] if len(sys.argv) > 4 else "cargo-cov.json"

    rust_passed, rust_failed, rust_ignored = parse_cargo_test_output(cargo_log)
    web_passed, web_failed, web_ignored = parse_vitest_output(vitest_log)
    py_passed, py_failed, py_ignored = parse_unittest_output(python_log)

    rust_coverage_pct = 0.0
    if os.path.exists(cov_json):
        try:
            with open(cov_json, "r") as f:
                cov_data = json.load(f)
                totals = cov_data.get("data", [{}])[0].get("totals", {})
                lines = totals.get("lines", {})
                rust_coverage_pct = lines.get("percent", 0.0)
        except Exception as e:
            print(f"Error parsing {cov_json}: {e}", file=sys.stderr)

    total_passed = rust_passed + py_passed + web_passed
    total_failed = rust_failed + py_failed + web_failed
    total_ignored = rust_ignored + py_ignored + web_ignored
    total_tests = total_passed + total_failed + total_ignored

    status_badge = "✅ PASSED" if total_failed == 0 else "❌ FAILED"

    summary_box = f"""
┌──────────────────────────────────────────────────────────────────────────────┐
│                        🧪 TEST RESULTS & COVERAGE SUMMARY                     │
├──────────────────────────────────────────────────────────────────────────────┤
│  Overall Status : {status_badge:<58} │
│  Total Tests    : {total_tests:<58} │
├──────────────────────────────────────────────────────────────────────────────┤
│  SUITE               │ PASSED │ FAILED │ IGNORED │ COVERAGE                  │
├──────────────────────┼────────┼────────┼─────────┼───────────────────────────┤
│  Rust Server         │ {rust_passed:<6} │ {rust_failed:<6} │ {rust_ignored:<7} │ {rust_coverage_pct:>5.1f}%                    │
│  Web UI Panel        │ {web_passed:<6} │ {web_failed:<6} │ {web_ignored:<7} │ N/A                       │
│  Python Test Suite   │ {py_passed:<6} │ {py_failed:<6} │ {py_ignored:<7} │ N/A                       │
├──────────────────────┼────────┼────────┼─────────┼───────────────────────────┤
│  TOTAL               │ {total_passed:<6} │ {total_failed:<6} │ {total_ignored:<7} │ {rust_coverage_pct:>5.1f}% (Rust)             │
└──────────────────────────────────────────────────────────────────────────────┘
"""
    print(summary_box)

    github_summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if github_summary_path:
        markdown_summary = f"""
### 🧪 Test Results & Coverage Summary

| Suite | Status | Passed | Failed | Ignored | Coverage |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Rust Server** | {'✅' if rust_failed == 0 else '❌'} | {rust_passed} | {rust_failed} | {rust_ignored} | **{rust_coverage_pct:.1f}%** |
| **Web UI Panel** | {'✅' if web_failed == 0 else '❌'} | {web_passed} | {web_failed} | {web_ignored} | N/A |
| **Python Tests** | {'✅' if py_failed == 0 else '❌'} | {py_passed} | {py_failed} | {py_ignored} | N/A |
| **TOTAL** | {status_badge} | **{total_passed}** | **{total_failed}** | **{total_ignored}** | **{rust_coverage_pct:.1f}%** |

```
{summary_box.strip()}
```
"""
        with open(github_summary_path, "a") as f:
            f.write(markdown_summary)

if __name__ == "__main__":
    format_summary()
