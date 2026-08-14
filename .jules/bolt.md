# Bolt's Journal - Critical Learnings

## 2026-03-30 - Intl.DateTimeFormat Instantiation Overhead in Timezone Formatting
**Learning:** Instantiating `Intl.DateTimeFormat` inside a loop over hundreds of timezones causes severe CPU overhead (~90ms+ per run). Caching `Intl.DateTimeFormat` instances per IANA timezone identifier reduces formatting loop times by ~80%+.
**Action:** Always cache `Intl.DateTimeFormat` formatters when repeatedly formatting dates across a known set of timezones or locales.
