## 2025-05-18 - Copy Button Visual & Accessible Feedback
**Learning:** Copy buttons in admin interfaces should provide immediate visual (checkmark icon swap) and screen-reader accessible (`aria-label` update) confirmation upon action, resetting via state timeout.
**Action:** Use the shared `<CopyButton value={...} label={...} />` component across all pages (S3 credentials, Replica tokens, MCP tokens/configs, and setup URLs) rather than inline un-feedbacked `<button>` or `<IconButton>` handlers.
