# Public download clients

## Status

Accepted for version 0.2.0.

## Context

Desktop launchers and other distributed clients cannot keep a reusable application
secret confidential. A token embedded in an executable, configuration template, or
public repository can be extracted and reused by anyone.

## Decision

The Server exposes a `downloader` application-credential preset containing only the
scopes required to request and consume an access package. It never includes object
write access.

The preset is suitable for local examples and public, non-sensitive downloads when
the credential is injected at runtime and the Origin applies rate limits and narrow
bucket policies. The token must not be committed or compiled into an application.

Protected downloads must use an external identity flow, such as OAuth 2.0/OIDC or an
application backend that authenticates the user and exchanges that identity for a
short-lived Ponte Mesh credential. Ponte Mesh Server does not treat a public client
identifier as authentication and does not mint anonymous bearer tokens.

## Consequences

- The administrative UI defaults new launcher credentials to least privilege.
- The full-access preset remains explicit for trusted server-side integrations.
- Example applications read credentials from the environment or an ignored local
  configuration file.
- Deployments serving protected content must integrate an identity provider or token
  broker instead of distributing a permanent secret.
