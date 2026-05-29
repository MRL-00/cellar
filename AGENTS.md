# AGENTS.md

Guidance for coding agents working in this repository.

## Project

Cellar is an MIT-licensed, open-source, cross-platform desktop database client.
It targets developers, DBAs, and analysts who need a fast DataGrip/TablePlus/DBeaver-class tool for browsing schemas, querying, editing result sets, and reviewing database changes before they are committed.

The product principles are:

- Desktop-first. There is no web-hosted v1 target.
- No telemetry by default. Do not add network reporting, analytics, or crash upload without explicit opt-in UX.
- BYO AI provider. Cellar must not proxy AI requests through a hosted Cellar service.
- AI is workflow-native: schema-aware editor completion, transparent context chips, generated SQL that is inspectable and gated before execution.
- The grid is core product surface. It must be virtualized, editable, honest about pending versus committed data, and safe around transactions.
- Extensibility matters. Drivers, AI providers, exporters, and renderers should be pluggable rather than hardcoded into app-only paths.

Read `SPEC.md` before making product or architectural changes. It is the canonical spec.

## Current Status

This repo is pre-alpha, but it is no longer just a static shell. Treat it as a working vertical slice with Postgres-only database connectivity, typed IPC, live schema browsing, and a read-only table-data path.

Implemented today:

- Tauri 2 + React 18 + TypeScript + Vite desktop shell.
- Tailwind v4 utility styling plus CSS design tokens in `apps/desktop/src/styles/tokens.css`.
- Zustand stores for connection, tab, and status state.
- `cellar-core` shared Rust contracts for engines, connections, schemas, queries, typed cell values, and errors.
- `tauri-specta` command registration and generated TypeScript bindings in `packages/ipc/src/generated.ts`.
- Tauri commands for listing/saving/deleting/testing/connecting/disconnecting connections, schema introspection, and query execution.
- OS-keychain credential storage in `cellar-secrets`; encrypted file fallback is documented but not built.
- First-party Postgres driver crate at `crates/cellar-drivers/postgres`.
- Live Postgres connection management, schema introspection, sidebar tree, table tabs, and read-only table loading.
- `@cellar/data-grid` package with editable-cell UI, pending-change state, filter chips, sticky/frozen-column styling, and commit/revert hooks.
- Settings, command palette, connection dialog, commit-preview modal, empty state, resizable panels, and title-bar window behavior.
- Scaffold tests/gates for Rust contracts, IPC, package lint/test tasks, and TypeScript typechecking.

Not implemented yet:

- MySQL, SQLite, SQL Server, and Azure SQL drivers.
- SQL editor / CodeMirror integration.
- Query cancellation, query history, execution plans, and streaming result events.
- Server-side grid pagination, virtualization, sorting, and type-aware filtering at scale.
- Real pending edit diff/review/commit execution through `cellar-diff`; current grid edits are local UI state only.
- AI providers and context pipeline.
- Plugin runtime.
- Broad unit/integration/e2e coverage, CI, signing, updater, and release packaging.

## Architecture

Workspace layout:

- `apps/desktop/` - Tauri app shell, React frontend, Rust app entrypoint.
- `crates/cellar-core/` - shared traits, errors, schema/query types.
- `crates/cellar-drivers/` - driver workspace root; Postgres lives in `crates/cellar-drivers/postgres/`.
- `crates/cellar-sql/` - SQL parsing, formatting, dialect support.
- `crates/cellar-diff/` - pending grid edits to transactional SQL.
- `crates/cellar-secrets/` - OS keychain and encrypted fallback credential storage.
- `crates/cellar-plugin-host/` - plugin process/runtime host.
- `packages/ui/` - shared UI primitives and design tokens.
- `packages/data-grid/` - custom virtualized grid package.
- `packages/sql-editor/` - CodeMirror 6 SQL editor wrapper.
- `packages/ipc/` - generated TypeScript bindings from Rust commands.
- `packages/ai/` - AI provider adapters, prompts, context building.
- `packages/plugin-sdk/` - plugin authoring interfaces and helpers.

The intended runtime is:

React frontend -> typed Tauri IPC -> Rust command/state layer -> driver traits in `cellar-core` -> concrete drivers in `cellar-drivers`.

Current query execution is materialized: the Postgres driver uses `fetch_all`, applies a host-side cap, and reports `truncated`. Large query results should still be moved to Tauri events keyed by query ID before this is considered production-safe. Do not add new APIs that require loading huge result sets into memory at once.

## Build And Validation

Common checks from the repo root:

```bash
pnpm install
pnpm typecheck
pnpm build
cargo check --workspace
cargo test --workspace
pnpm lint
pnpm test
```

Run `pnpm install` after pulling `main` when package dependencies or workspace links have changed. Stale `node_modules` commonly shows up as missing `@cellar/*`, `zustand`, or Tailwind plugin types.

Current caveat: `pnpm lint` and `pnpm test` run real checks, but they are not substitutes for future ESLint/Vitest/Playwright coverage once the app has more behavior to protect.

Use Clawpatch for automated review when preparing substantial work:

```bash
clawpatch doctor
clawpatch review --include-dirty
```

If Clawpatch reports findings, triage them before claiming the project is ready to ship.

## Coding Rules

- Keep changes scoped. Avoid broad refactors while the architecture is still being made real.
- Prefer repo patterns over inventing new ones.
- Use `rg` for searches.
- Use `apply_patch` for manual edits.
- Do not revert user changes.
- Do not commit generated build artifacts such as `dist/` or `target/`.
- Add dependencies only when they match the spec or are clearly necessary for the current slice.
- Keep frontend TypeScript strict.
- Keep Rust errors typed; avoid stringly-typed backend contracts.
- Generated IPC types should come from Rust command/type definitions, not hand-maintained duplicates.
- After changing Tauri commands or Rust IPC-facing types, regenerate `packages/ipc/src/generated.ts` with the desktop codegen binary before updating frontend call sites.
- Do not hand-build executable SQL in React components. Use typed Rust/SQL/diff builders for execution paths; UI previews must at least quote identifiers and escape literals safely until the shared builder exists.
- Do not render stub controls as if they work. Unimplemented buttons, toggles, and settings must be disabled/read-only or wired to real state and callbacks.
- Pull request titles must not use `codex:` or `[codex]` prefixes. Use conventional prefixes such as `feat:`, `fix:`, `bug:`, `chore:`, `docs:`, `test:`, `build:`, `ci:`, or `refactor:`.
- No human-authored source, documentation, or configuration file may exceed 800 lines. If a file approaches that size, split it by responsibility before adding more code. Generated lockfiles and binary assets are exempt, but do not hand-edit them except through their owning tools.

## Security And Privacy

- Never write database credentials to plain-text config.
- Store credentials through `cellar-secrets`: OS keychain first. The encrypted fallback is documented in `docs/architecture/adr/0001-secret-fallback.md` but not implemented yet.
- Connection configs are persisted under `~/.cellar/connections.json`; that file must never contain passwords or secrets.
- Keep Tauri capabilities narrow. Frontend must not gain arbitrary shell or file access.
- Do not send credentials to AI providers.
- Do not reuse AI provider keys for unrelated app encryption, sync, telemetry, or account features.
- AI context must be inspectable before sending.
- Destructive SQL needs explicit confirmation, especially on production-tagged connections.
- Production-tagged connections should be visually distinct and may default to read-only.

## UI And Product Notes

- The UI should be dense, technical, and work-focused.
- Dark theme is default. Light theme should remain possible.
- Tailwind v4 is installed and used for most component styling. Design tokens still live in `apps/desktop/src/styles/tokens.css`.
- Keep repeated modal/settings UI split into small files under `apps/desktop/src/components/modals/`; do not grow single modal files past the 800-line rule.
- Use keyboard-first interaction patterns. The command palette is `Cmd+K`.
- Use monospace for SQL, identifiers, and data.
- Avoid telemetry, cloud sync, collaboration, ETL/job orchestration, and admin/cluster-management features for v1.

## Suggested Implementation Order

Best next vertical slices from the current state:

1. Stabilize the Postgres vertical slice: integration setup, clearer errors, reconnect behavior, query cancellation, and row-limit safety.
2. Move query results from materialized `fetch_all` to streamed/page-style Tauri events before chasing huge-grid performance.
3. Add the CodeMirror SQL editor and wire run-current-statement / run-selection through existing typed IPC.
4. Implement `cellar-diff` for generated transactional SQL, then connect the grid's pending edits to a real Review & Commit path.
5. Add server-side sort/filter/pagination and real virtualization to the data grid.
6. Broaden tests around `cellar-core`, Postgres introspection/decoding, Tauri command contracts, and the grid's pending-change behavior.

Do not prioritize AI providers, plugin runtime, or multi-engine breadth until the Postgres query/edit spine is stable and tested.

## Known Spec Gaps To Resolve

- `SPEC.md` describes plugin loading two ways: dynamic libraries/C ABI in one section, out-of-process JSON-RPC later. Prefer out-of-process JSON-RPC for crash isolation unless an ADR changes it.
- `SPEC.md` references docs that do not exist yet, including `docs/keymap.md`, `docs/architecture/open-questions.md`, driver authoring docs, plugin authoring docs, and `CONTRIBUTING.md`.
- Tauri config currently has `csp: null`; this is not acceptable for a security-ready build.
