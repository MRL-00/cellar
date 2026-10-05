# OpenAI local extension boundary

Date: 2026-10-03 UTC

## Context

Cellar main `4611d97be69c6e26f312f198eba8a0d6dc620081` uses native Rust/GPUI. Browser hosts cannot render GPUI. The user requires connection, browse/filter, SQL and reviewed-edit workflows, with their own databases and selected current-chat SQLite attachments.

## Decision

Add optional MCP App `apps/cellar-browser` and constrained Rust executable `crates/cellar-extension-service`. Reuse native connection contracts, SQL quoting/statement/cursor helpers, diff planning and extension-specific keychain identities. Preserve native renderer and driver behavior.

Use sqlx in this adapter rather than exposing unrestricted native write paths. Reads remain structurally validated, read-only and bounded. Only a distinct commit action permits reviewed insert/update/delete transactions when a saved profile allows editing. Preview digests bind exact changes and row revisions; row locks and transactions reject conflicts and roll back failures.

Profiles are separate, locked and atomically replaced. Passwords enter app-only tools, pass via stdin and use new OS keychain identities. Remote profiles require verified TLS. Supported host tools materialize explicitly selected chat attachments, then create private snapshots. The browser has no shell or chat-history access. Import/download are chunked and bounded.

## Consequences

No new GPUI IPC layer or removed Tauri runtime is revived. Browser workflows are reconstructed rather than sharing the renderer. Process-per-request isolation simplifies cancellation at reconnect cost. UI metadata avoids automatic model context; the host still transports rows.

Complete native parity is not claimed: unrestricted SQL writes, migrations, ER diagrams, SSH, advanced grid features and persistent workspace/history remain absent. File entrypoints depend on host support. Local Codex can access authorized Mac paths/private servers; public ChatGPT requires an independent approved endpoint and security architecture.

Trusted schemas and least-privilege roles remain necessary; database-side effects exceed syntax validation. Large fields allocate before response truncation, pending edits are in memory, and cancelled commits can require outcome inspection before retry.
