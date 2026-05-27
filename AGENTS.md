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

This repo is pre-alpha scaffolding. Treat the visible desktop UI as a static shell unless you have verified otherwise.

Implemented today:

- Tauri 2 + React 18 + TypeScript + Vite desktop shell.
- Static frontend layout: title bar, sidebar, tab strip, workspace placeholder, bottom panel, AI panel, status bar.
- Cargo and pnpm workspaces.
- Placeholder Rust crates and TypeScript packages.

Not implemented yet:

- Tauri command layer and generated IPC bindings.
- Real connection management.
- Credential storage.
- Postgres/MySQL/SQLite/SQL Server/Azure SQL drivers.
- Schema introspection.
- SQL editor.
- Query execution and cancellation.
- Data grid.
- Pending edit diff/review/commit flow.
- AI providers and context pipeline.
- Plugin runtime.
- Meaningful tests, linting, CI, signing, updater, or packaging gates.

## Architecture

Workspace layout:

- `apps/desktop/` - Tauri app shell, React frontend, Rust app entrypoint.
- `crates/cellar-core/` - shared traits, errors, schema/query types.
- `crates/cellar-drivers/` - first-party database drivers.
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

Large query results should stream through Tauri events keyed by query ID. Do not design APIs that require loading huge result sets into memory at once.

## Build And Validation

Common checks from the repo root:

```bash
pnpm typecheck
pnpm build
cargo check --workspace
cargo test --workspace
pnpm lint
pnpm test
```

Current caveat: `pnpm lint` and `pnpm test` are scaffold-era gates. They run real checks, but they are not substitutes for future ESLint/Vitest/Playwright coverage once the app has behavior to protect.

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
- Pull request titles must not use `codex:` or `[codex]` prefixes. Use conventional prefixes such as `feat:`, `fix:`, `bug:`, `chore:`, `docs:`, `test:`, `build:`, `ci:`, or `refactor:`.

## Security And Privacy

- Never write database credentials to plain-text config.
- Store credentials through `cellar-secrets` once implemented: OS keychain first, encrypted fallback only with explicit user-controlled master password.
- Keep Tauri capabilities narrow. Frontend must not gain arbitrary shell or file access.
- Do not send credentials to AI providers.
- AI context must be inspectable before sending.
- Destructive SQL needs explicit confirmation, especially on production-tagged connections.
- Production-tagged connections should be visually distinct and may default to read-only.

## UI And Product Notes

- The UI should be dense, technical, and work-focused.
- Dark theme is default. Light theme should remain possible.
- Spec says design tokens belong in `packages/ui/src/tokens/`; current scaffold keeps them in `apps/desktop/src/styles/tokens.css`.
- The spec calls for CSS variables plus selective shadcn/ui source ownership. Tailwind is listed in the spec but is not currently installed.
- Use keyboard-first interaction patterns. The command palette is `Cmd+K`.
- Use monospace for SQL, identifiers, and data.
- Avoid telemetry, cloud sync, collaboration, ETL/job orchestration, and admin/cluster-management features for v1.

## Suggested Implementation Order

The best next vertical slice is:

1. Define core Rust types and traits in `cellar-core`.
2. Add typed Tauri command modules under `apps/desktop/src-tauri/src/commands/`.
3. Generate/import TypeScript IPC bindings through `packages/ipc`.
4. Implement minimal Postgres connection storage, connect, schema introspection, and simple query execution.
5. Replace static sidebar data with real connection/schema state.
6. Add a basic CodeMirror editor and read-only results grid.
7. Add tests around every backend contract before expanding engines or edit flows.

Do not start with the AI panel, plugin marketplace, or multi-engine breadth before the connection/query spine works end-to-end.

## Known Spec Gaps To Resolve

- `SPEC.md` describes plugin loading two ways: dynamic libraries/C ABI in one section, out-of-process JSON-RPC later. Prefer out-of-process JSON-RPC for crash isolation unless an ADR changes it.
- `SPEC.md` references docs that do not exist yet, including `docs/keymap.md`, `docs/architecture/open-questions.md`, driver authoring docs, plugin authoring docs, and `CONTRIBUTING.md`.
- Tauri config currently has `csp: null`; this is not acceptable for a security-ready build.
