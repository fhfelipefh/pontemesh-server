# Software release versioning and launcher credentials

## Status

Accepted for version 0.10.8.

## Context

Desktop game launchers and distributed client applications need to determine whether a newer version of a specific application is available without executing heavy catalog listing operations. Executing full bucket listings or scanning thousands of objects incurs high server and network overhead, leaks directory topology, and requires elevated permissions that client launchers should never possess.

Furthermore, different games and software projects rely on distinct versioning paradigms (such as Semantic Versioning, monotonic build numbers, release channels, or alphanumeric tags). If a bucket's versioning scheme could be mutated dynamically after launchers have been distributed to end users, client-side comparison logic could break unexpectedly in production.

## Decision

1. **Dedicated update check endpoint**: The Origin exposes `GET /pontemesh/updates/{bucket_name}/{software_id}?current={version}`, which evaluates keys matching `{software_id}/...` and returns an ultra-lightweight decision payload (`hasUpdate`, `latestVersion`, `currentVersion`, `latestKey`, `versioningScheme`).
2. **Immutable versioning scheme per bucket**: Each bucket defines `release_versioning_scheme` (`DISABLED`, `SEMVER`, `BUILD_NUMBER`, `CHANNEL`, `TAG`). Once configured to an active scheme, it is immutable in-place. Operators who wish to migrate an application to a new versioning paradigm must provision a new bucket, guaranteeing that existing deployed launchers do not encounter unexpected parsing failures.
3. **Hyper-scoped `launcher` credential preset**: The server provides a `launcher` preset containing only `pontemesh:update:check`, `pontemesh:access-package:create`, `pontemesh:manifest:read`, `pontemesh:sources:read`, and `pontemesh:availability:read`. Object writing (`origin:objects:write`) and bucket browsing are completely omitted.

## Consequences

- Game and software launchers can verify updates with minimal network latency and minimal load on the central Origin.
- Versioning scheme immutability prevents operational breaking changes across distributed client fleets.
- Launcher credentials adhere strictly to least-privilege access, eliminating write access or catalog discovery risks from client devices.
- The Origin retains centralized control: manifest validation, access package issuance, and fragment hash integrity remain fully enforced during downloads.
