# Ponte Mesh Server Updates

The server update flow is prepared for GitHub Releases but does not apply a new
server binary automatically. It can check, download, and verify a newer release
in the background, then leave the staged artifact for an operator-controlled
restart or rollout.

## Release publishing

Use the manual `Server Release` GitHub Actions workflow after the repository is
public and the release version is already committed in `Cargo.toml`.

The workflow:

- accepts a semver version without the `v` prefix;
- refuses to publish if the caller is not the repository owner;
- refuses to publish if `Cargo.toml` does not match the requested version;
- builds the web panel once and native server binaries for Linux x64, Windows
  x64, macOS Intel, and macOS ARM;
- publishes each server package, its SHA-256 file, and
  `pontemesh-server-v<VERSION>-manifest.json`.

## Update checking

First verified use:

```bash
./scripts/check-server-update.sh \
  --repository OWNER/REPOSITORY \
  --trust-on-first-use \
  --stage \
  --asset-pattern '*windows-x64.zip'
```

Regular background check:

```bash
./scripts/check-server-update.sh \
  --repository OWNER/REPOSITORY \
  --stage \
  --asset-pattern '*windows-x64.zip' \
  --background
```

Use `*linux-x64.tar.gz`, `*macos-x64.tar.gz`, or
`*macos-arm64.tar.gz` as the asset pattern on those platforms.

By default, checks are spaced by 24 hours. State and reports are written under
`target/update-state`; staged downloads are written under
`target/update-staging`. Override those paths with `PONTEMESH_UPDATE_STATE_DIR`
and `PONTEMESH_UPDATE_STAGE_DIR`.

Security boundaries:

- repository trust is pinned locally on first verified use;
- switching repositories fails unless the local state is intentionally replaced;
- downloaded assets must match the manifest product, version, size, and SHA-256;
- the updater never stops, replaces, or starts the server process.

## Applying a staged server update

A running process cannot safely replace itself in place. The prepared update is
therefore staged without service interruption, and application should be done by
the service manager or deployment layer.

For reduced downtime, run the new server version beside the old one, point the
reverse proxy or service manager to the new instance after health checks pass,
then stop the old instance. Without an external supervisor or proxy, the final
binary switch still requires a controlled restart.
