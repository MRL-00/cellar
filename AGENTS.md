# AGENTS.md

Guidance for coding agents working in this repository.

## Project

Cellar is an MIT-licensed, open-source, cross-platform desktop database client for developers, DBAs, and analysts. It is a dense DataGrip/TablePlus/DBeaver-class workspace for browsing schemas, querying, editing result sets, and reviewing changes before commit.

Product principles:

- Desktop-first. There is no web-hosted product target.
- No telemetry, crash upload, or network reporting without explicit opt-in UX.
- BYO AI provider. Cellar must not proxy AI through a hosted Cellar service.
- AI context and generated SQL must be inspectable and gated before execution.
- The grid is core product surface: bounded, virtualized, editable, and transaction-safe.
- Drivers, providers, exporters, and renderers should have coherent extension boundaries.

Read `SPEC.md` before product or architecture changes. It is canonical.

## Current architecture

Cellar 1.0 is a native Rust application. The former Tauri/React desktop client has been removed.

- `apps/desktop-gpui/` — production GPUI shell, editor, grid, and native assets.
- `apps/site/` — React/Vite marketing and download site; it is not app code.
- `crates/cellar-runtime/` — application, connection, query, history, and transaction services.
- `crates/cellar-core/` — shared traits, typed errors, schema/query contracts, and cell values.
- `crates/cellar-drivers/` — concrete database drivers.
- `crates/cellar-sql/` — SQL parsing, formatting, and dialect behavior.
- `crates/cellar-diff/` — pending row edits to transactional SQL.
- `crates/cellar-schema-diff/` — schema comparison and migration generation.
- `crates/cellar-secrets/` — OS-keychain credential storage.
- `crates/cellar-ai/` — native provider transports and authentication.

Runtime shape:

```text
GPUI entities/models -> cellar-runtime -> cellar-core traits -> concrete drivers
```

GPUI consumes bounded query pages directly from Rust services. Do not introduce a second IPC boundary or APIs that materialize unbounded result sets.

A small read-only WebKit local-storage reader remains in `apps/desktop-gpui` solely to migrate layout and appearance preferences for users upgrading from pre-GPUI releases. Do not expand it into a compatibility client.

The release workflow retains `@tauri-apps/cli` only for its standalone minisign-compatible signer and the already-issued updater key format. It is not a desktop runtime dependency.

## Build and validation

Common checks from the repository root:

```bash
pnpm install
cargo check --workspace
cargo test --workspace
pnpm typecheck
pnpm build
pnpm lint
pnpm test
```

Desktop development and performance checks:

```bash
pnpm dev
pnpm build:native
pnpm perf:native:startup
pnpm perf:native:grid
```

Run `pnpm install` after package or workspace changes. `pnpm` validates the website and JS repository tooling; desktop behavior is covered by Cargo checks and tests.

Use Clawpatch for substantial work when available:

```bash
clawpatch doctor
clawpatch review --include-dirty
```

Triage actionable findings before calling work ready to ship.

## Coding rules

- Keep changes scoped and prefer existing patterns.
- Use `rg` for searches and `apply_patch` for manual edits.
- Never revert unrelated user or agent changes.
- Do not commit generated build outputs such as `dist/` or `target/`.
- Add dependencies only when necessary for the requested vertical slice.
- Keep frontend TypeScript strict and Rust errors typed.
- Keep executable SQL construction in typed Rust SQL/diff/driver builders, not GPUI view code or website components.
- Unimplemented controls must be visibly disabled/read-only, not presented as working.
- Pull request titles must use conventional prefixes and never `codex:` or `[codex]`.
- Human-authored source, documentation, and configuration files must stay below 800 lines. Split by responsibility before crossing the limit. Generated files, lockfiles, and binary assets are exempt.

## Security and privacy

- Never write database or provider credentials to plain-text configuration.
- Store credentials through `cellar-secrets`; `~/.cellar/connections.json` must contain metadata only.
- Do not expose arbitrary shell or filesystem capability to feature code.
- Do not send credentials to AI providers or include them in AI context.
- Do not reuse provider keys for encryption, sync, telemetry, or unrelated features.
- AI context must remain inspectable before sending.
- Destructive SQL needs explicit confirmation, especially on production-tagged connections.
- Production connections must be visually distinct and may default to read-only.

## UI and product notes

- Keep the native UI dense, technical, and work-focused.
- Dark theme is default; preserve light-theme support.
- Native design tokens live in `apps/desktop-gpui/src/theme.rs`.
- Prefer keyboard-first interactions; the command palette is Cmd/Ctrl+K.
- Use monospace for SQL, identifiers, and data.
- Maintain bounded row and column rendering in the grid.
- Avoid telemetry, cloud sync, collaboration, ETL/job orchestration, and cluster administration.

## Known gaps

- External plugin loading and its SDK are planned but not implemented. Prefer an out-of-process JSON-RPC design for crash and ABI isolation unless an ADR changes it.
- Semantic accessibility for custom GPUI controls still needs broader platform coverage.
- Windows and Linux optimized builds are checked in CI, but signed release packaging currently ships macOS Apple Silicon only.
- Several contributor and driver/plugin authoring documents referenced by `SPEC.md` do not exist yet.
