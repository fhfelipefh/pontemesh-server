# Version Bumping Rule

Whenever you make any changes to the source code (backend or frontend), you MUST always increment the patch version of the project in ALL necessary places to ensure the update is properly tracked and released across the entire monorepo.

- Increment the `version` field in `Cargo.toml` and run `cargo check` to update `Cargo.lock`.
- Increment the `version` field in `web/package.json` and run `npm install` in `web/` to update `package-lock.json`.
