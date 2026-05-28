# ADR 0001: Encrypted-file fallback for connection secrets

- **Status:** Open
- **Date:** 2026-05-28

## Context

`cellar-secrets` writes connection passwords to the OS keychain on macOS,
Windows, and Linux. Some environments will not have a usable keychain:

- Headless Linux CI containers without a Secret Service daemon.
- Sandboxed app containers where the keychain entitlement isn't granted.
- Bring-your-own-laptop setups where the user has explicitly disabled the
  Secret Service.

SPEC §5.3 and §7 both call for a "file encrypted with a key derived from a
user-supplied master password" as the fallback. That is not built in this
slice — the goal here was to land the live-connection loop without scope
creep.

## Decision (deferred)

The first vertical slice ships only the keychain path. Connections that fail
to open the keychain return a typed `CellarError::Authentication` and the UI
surfaces a clear "couldn't store password — please retry or open a bug"
message rather than silently writing plaintext anywhere.

## Open questions

- Which KDF? Argon2id (modern, slower) vs. scrypt (older, simpler). Lean
  Argon2id.
- Where does the encrypted file live? Likely `~/.cellar/secrets.kdb`.
- How is the master password collected on app launch — modal blocker, or
  lazy on first secret access?
- Do we keep a per-connection IV or a single salt? Per-connection IV avoids
  any chosen-plaintext concerns at the cost of more bookkeeping.

## Consequences

Until this is built, Cellar will not work on systems without a functioning
OS keychain. That is acceptable for v0.x but must be resolved before any
1.0 release.
