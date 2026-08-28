# Cellar — Specification

**Status:** Draft, v1.0 target
**Last updated:** 2026-05-26
**Owner:** [@nz_mrl](https://x.com/nz_mrl)
**License:** MIT

---

## 1. Overview

Cellar is an open-source, cross-platform desktop database client. It targets the same audience as JetBrains DataGrip, TablePlus, and DBeaver: developers, DBAs, and analysts who work with multiple databases daily and need a fast, ergonomic environment for browsing, querying, editing, and shipping schema and data changes.

Cellar's positioning is shaped by three convictions:

1. **The existing tools are either heavy (DataGrip, DBeaver), closed-source (TablePlus), or thin (Beekeeper, pgAdmin).** There is room for a fast, modern, open-source client that takes the DataGrip feature set seriously.
2. **AI belongs in the workflow, not stapled to the side.** Schema-aware completion in the editor, context-aware chat, and explainable SQL generation should be a first-class part of the product.
3. **Extensibility is a feature.** Drivers, AI providers, exporters, and data renderers should all be pluggable so the community can ship what they need without waiting on the core team.

Cellar is MIT licensed. There is no paid tier. There is no telemetry without opt-in.

---

## 2. Goals and non-goals

### Goals

- Connect to PostgreSQL, MySQL, SQLite, SQL Server, and Azure SQL on day one.
- Support row, column, and cell-level editing of result sets with reviewable, transactional commits.
- Provide a SQL editor with autocomplete, lint, format, execution plans, and history.
- Provide schema introspection and navigation (tables, views, functions, procedures, indexes, foreign keys).
- Integrate AI assistance with full schema context, BYO API key, and a clear separation between read-only and destructive actions.
- Be fast and light enough to run alongside an IDE, browser, and Slack without complaint.
- Be extensible via a documented plugin API (drivers, AI providers, exporters).
- Ship signed, auto-updating binaries for macOS, Windows, and Linux.

### Non-goals (for v1.0)

- Cloud sync, team collaboration, or shared workspaces.
- A web-hosted version. Cellar is desktop-first.
- ETL, ELT, or scheduled job orchestration. Cellar is a client, not a pipeline.
- Database administration features beyond what a developer needs day-to-day (no replication setup, no backup orchestration, no cluster management).
- Mobile or tablet support.

### Explicit anti-goals

- Telemetry-by-default.
- A "Pro" tier that locks features behind a paywall.
- A walled-garden plugin marketplace.
- Closed-source drivers or AI providers shipped by the core team.

---

## 3. Target users

- **Backend engineers** running local Postgres and remote staging/prod environments, switching between schemas dozens of times a day.
- **DBAs and platform engineers** managing SQL Server and Azure SQL alongside Postgres, who care about execution plans and transactional safety.
- **Analysts** who write SQL but want autocomplete and AI help to move faster.
- **Open-source contributors** who want a tool they can extend, not just use.

Cellar is **not** aimed at non-technical business users. The UI assumes you understand SQL and database concepts.

---

## 4. Stack

| Layer | Choice | Reason |
|---|---|---|
| Desktop shell and UI | GPUI 0.2.2 (exact pin) | GPU-rendered native Rust UI with direct access to shared typed services |
| Build | Cargo | One production language and build graph for the desktop client |
| State | GPUI entities plus plain Rust models | UI invalidation stays local and domain state remains testable without a window |
| Styling | GPUI styles plus shared Rust theme tokens | Dense token-driven UI without a browser layout engine |
| Component primitives | Owned GPUI components | Cellar controls focus, accessibility, and rendering cost |
| Data grid | Custom GPUI grid with two-axis virtualization | The grid is the product — visible cells must stay bounded |
| SQL editor | Native GPUI editor backed by `cellar-sql` | Native focus/input with shared dialect parsing and formatting |
| Backend | Rust (stable toolchain) | Performance, safety, best-in-class DB libraries |
| Async runtime | Tokio | Standard |
| DB drivers | `sqlx` (Postgres, MySQL, SQLite), `tiberius` (SQL Server) | Mature, async, dialect-aware |
| Credential storage | OS keychain via `keyring` crate, fallback to encrypted file | Standard, secure |
| AI providers | Rust provider adapters behind typed runtime services | Provider keys and OAuth credentials never enter UI state |
| Telemetry | None by default. Opt-in only, self-hosted endpoint configurable. | Trust |
| Package management | Cargo workspace; pnpm retained only during migration | Tauri/React stays buildable until verified parity |
| Testing (Rust) | Built-in + `insta` for snapshots | Standard |
| Testing (legacy TS) | Vitest + Playwright | Protects the old client until its removal gate passes |
| Linting | `cargo clippy`, `rustfmt`; legacy `eslint` and `prettier` | Standard |
| CI | GitHub Actions | Free for OSS |
| Release | Signed Cargo-built GPUI bundles for macOS, Windows, and Linux | Preserve existing platform support |

---

## 5. Architecture

### 5.1 High-level

```
┌─────────────────────────────────────────────────────────────┐
│                       GPUI Window                            │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                    Native Rust UI                       │  │
│  │                                                        │  │
│  │  Sidebar │  Tabbed Editor / Grid  │  AI Panel          │  │
│  │          │  Results / Plan / etc. │                    │  │
│  └───────────────────────────────────────────────────────┘  │
│                          │                                   │
│              bounded typed runtime calls/pages               │
│                          │                                   │
│  ┌───────────────────────────────────────────────────────┐  │
│  │             Shared `cellar-runtime` services           │  │
│  │  ┌──────────┬───────────┬────────────┬──────────────┐ │  │
│  │  │ UI tasks │ app state │ connection │  driver host │ │  │
│  │  │ + pages  │  manager  │   pool     │              │ │  │
│  │  └──────────┴───────────┴────────────┴──────────────┘ │  │
│  │                          │                             │  │
│  │  cellar-core (traits) ───┴─── cellar-drivers           │  │
│  │  cellar-sql (parsing, formatting)                      │  │
│  │  cellar-diff (pending changes → transactional SQL)     │  │
│  │  cellar-secrets (credential storage)                   │  │
│  │  cellar-plugin-host (external driver loading)          │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                  ┌───────────┴───────────┐
                  │  PG │ MySQL │ SQLite  │
                  │  MSSQL │ Azure SQL    │
                  └───────────────────────┘
```

### 5.2 Repository layout

The repo uses Cargo for the production client and Rust services. The pnpm
workspace remains only while the Tauri/React client is the parity reference.

```
cellar/
├── apps/desktop-gpui/           # Production GPUI desktop client
├── apps/desktop/                # Legacy parity reference during migration
├── crates/                      # Rust workspace
│   ├── cellar-runtime/          # Shared application and connection services
│   ├── cellar-core/             # Traits, errors, shared types
│   ├── cellar-drivers/          # Per-engine drivers
│   ├── cellar-sql/              # SQL parsing, dialect handling
│   ├── cellar-diff/             # Pending changes → SQL
│   ├── cellar-secrets/          # Credential storage
│   └── cellar-plugin-host/      # External plugin loading
├── packages/                    # pnpm workspace
│   ├── ui/                      # Shared component library
│   ├── data-grid/               # The grid (its own package)
│   ├── sql-editor/              # CodeMirror wrapper
│   ├── ipc/                     # Generated TS bindings from Rust commands
│   ├── ai/                      # AI providers, prompts, context building
│   └── plugin-sdk/              # SDK for community plugin authors
├── plugins/                     # First-party plugins
├── docs/                        # Architecture, ADRs, contributor guides
├── examples/                    # Docker compose for local DBs, sample data
└── scripts/                     # Build, codegen, release helpers
```

See `docs/architecture/overview.md` for the full structure once committed.

### 5.3 Rust crate responsibilities

**`cellar-core`** owns the contracts. Every driver implements traits from here. Frontend types are generated from here via specta. Contains:

- `Driver` trait — connect, introspect, execute, transaction lifecycle
- `Connection` trait — represents an open connection
- `Schema` types — database, schema, table, column, index, constraint, view, function, procedure
- `Query` and `QueryResult` types — including streaming row support
- `Transaction` trait — for grouped commits
- Error types — typed, not stringly-typed

**`cellar-drivers/*`** implement `Driver` per engine. First-party drivers:

- `postgres` — via `sqlx`
- `mysql` — via `sqlx`
- `sqlite` — via `sqlx`
- `sqlserver` — via `tiberius`
- `azure-sql` — wraps `sqlserver`, adds Azure AD / managed identity auth

Each driver is responsible for dialect-specific quirks (identifier quoting, type mapping, system catalog queries, transaction syntax).

**`cellar-sql`** handles SQL parsing, formatting, and dialect awareness. Built on `sqlparser-rs`. Used for autocomplete context, formatting on save, and validating generated SQL before execution.

**`cellar-diff`** is the pending-changes engine. Takes a list of row-level edits from the grid (insert, update, delete) and emits a transactional SQL script. Handles:

- Composite primary keys
- NULL handling
- Unique constraint awareness (best-effort)
- Per-dialect syntax
- Optimistic concurrency: re-validates rows against the DB before commit

**`cellar-secrets`** stores connection credentials. Uses OS keychain (`keyring` crate) on macOS, Windows, and Linux where available. Falls back to a file encrypted with a key derived from a user-supplied master password.

**`cellar-plugin-host`** loads external drivers and providers. v1.0 uses dynamic libraries with a stable C ABI plus WASM as a future direction. See §10.

### 5.4 UI/runtime boundary

GPUI calls typed `cellar-runtime` services directly. UI tasks may own lightweight
handles and identifiers, but database connections, credentials, and transaction
state remain in the runtime. The legacy Tauri commands re-export the same runtime
while migration is in progress.

Services are grouped by feature: `connection`, `query`, `schema`, `transaction`,
`ai`, and `settings`.

Streaming results use bounded channels keyed by query ID. Large result sets arrive
in pages of N rows (default 500); no complete result is materialized at the UI
boundary.

### 5.5 State management

UI state is split across GPUI entities per feature:

- connections — connection list, status, active connection
- tabs — open editors and grids, focus, split state
- schema tree — introspection cache per connection
- AI — conversation state, context chips, provider config
- settings — preferences, themes, keymap

Domain models remain plain Rust structs where GPUI observation is unnecessary.
There is no second copy of backend connection or credential state in the UI.

---

## 6. Functional spec

### 6.1 Connections

#### Connection management

- Sidebar lists all configured connections, grouped by environment if tagged.
- Each connection has a colored engine badge (P / M / S / Sl / Az for Postgres / MySQL / SQL Server / SQLite / Azure SQL).
- Click to expand: shows databases → schemas → tables/views/functions/procedures.
- Right-click for context menu: edit, duplicate, disconnect, remove, set color tag.
- Status dot per connection: connected, disconnected, error.
- Connection-wide read-only toggle. Read-only connections cannot run destructive statements; the UI surfaces this prominently.

#### New connection dialog

Per-engine form. Common fields: host, port, database, user, password, SSL mode. Engine-specific fields:

- **Postgres:** schema search path, application name, connect timeout
- **MySQL:** charset, collation
- **SQLite:** file path or `:memory:`
- **SQL Server / Azure SQL:** instance name, trust server certificate, encrypt
- **Azure SQL only:** auth method (SQL auth, Azure AD password, Azure AD interactive, Managed Identity)

Optional: SSH tunnel (host, port, user, key file / password), HTTP proxy, custom JDBC-style connection string override.

Test connection button. Save to keychain.

#### Connection tagging

Each connection can have an environment tag: `local`, `dev`, `staging`, `prod`, or custom. Production-tagged connections:

- Show a red accent in the sidebar and tab strip
- Require a confirmation step before running any DDL or `DELETE` / `UPDATE` without `WHERE`
- Optionally enforce read-only by default with explicit unlock

### 6.2 Sidebar and schema tree

- Top: filter input that fuzzy-matches across all expanded nodes (`⌘F` global, scoped when tree is focused).
- Tree shows: databases → schemas → tables, views, functions, procedures, indexes (under their table), foreign keys (under their table).
- Tables show row count (lazy-loaded) and a small FK indicator if referenced by others.
- Right-click on a table: open data, open structure, generate SELECT/INSERT/UPDATE/DELETE, copy fully-qualified name, drop (with confirmation), refresh.
- Drag a table into the editor to insert its qualified name.
- Refresh button refetches schema. Schema introspection results are cached per connection until manually refreshed.

### 6.3 Tabbed workspace

- Tabs hold either a **SQL editor** or a **table grid**.
- Tabs are draggable to reorder, draggable out to split horizontally or vertically.
- Splits allow side-by-side: two queries, a query and a table, two tables.
- Each tab is scoped to a connection and a database. The active connection/db is shown in the tab header.
- Tab state (unsaved query, applied filters, pending edits) is preserved across app restarts.
- Tabs can be renamed.

### 6.4 SQL editor

#### Core features

- CodeMirror 6 with a custom SQL language pack per dialect.
- Autocomplete from live schema: tables, columns, functions. Context-aware (post-FROM suggests tables; post-WHERE suggests columns from referenced tables).
- Snippets: `sel`, `ins`, `upd`, `del`, `jln`, etc. User-definable.
- Format on save (via `cellar-sql`).
- Lint markers for obvious mistakes (missing FROM, ambiguous columns, deprecated syntax).
- Multi-cursor, column selection, find/replace with regex, fold.
- Bracket matching, indent guides, line numbers.

#### Execution

- `⌘Enter` runs the statement at the cursor.
- `⌘Shift+Enter` runs the entire file.
- Selecting text and `⌘Enter` runs the selection.
- Multiple statements run sequentially, each result in a tab below.
- Long-running queries can be cancelled.
- Estimated cost / row count shown before execution if available (via EXPLAIN).
- Query timeout configurable per connection.

#### History

- Every executed query is logged with timestamp, connection, duration, success/failure, row count.
- History is searchable.
- History is stored locally in SQLite (`~/.cellar/history.db`).

### 6.5 Data grid

The grid is the most important component in the product. It must handle large result sets, support full editing, and never lie about what's been committed.

#### Display

- Virtualized rendering. Smooth scroll through millions of rows.
- Column headers show name, type, nullability indicator, PK/FK badges.
- Sort by clicking column headers (client-side for small sets, server-side via `ORDER BY` for large).
- Filter bar above the grid. Per-column filter with type-aware operators (`=`, `!=`, `>`, `<`, `LIKE`, `IS NULL`, etc.).
- Column reorder, freeze, hide, autosize.
- Row numbers in a gutter.
- Footer shows: filtered count / total count, page info, refresh, paging controls.

#### Editing

- Click to select cell. Double-click or F2 to edit.
- Tab / Shift+Tab moves horizontally, Enter / Shift+Enter moves vertically.
- Type-aware editors: text input for strings, number input for numerics, date picker for dates, dropdown for enums, NULL toggle.
- Copy from Excel and paste into a range. Copy from grid into clipboard as TSV or CSV.
- Multi-row select with Shift+Click and ⌘+Click.
- Right-click row: duplicate row, delete row, set selected cells to NULL, copy as INSERT, jump to FK reference.

#### Pending changes

- Edits are local until committed. The grid maintains an in-memory diff.
- Edited rows have a tinted background:
  - Green for inserts
  - Yellow for updates
  - Red for deletes
- Edited cells have a small dot indicator and tooltip showing the original value.
- Footer shows: `4 pending · 1 insert · 2 updates · 1 delete · [Revert] [Review & Commit]`.

#### Review and commit

- "Review & Commit" opens a modal showing the generated SQL.
- The SQL is editable before execution.
- All statements are wrapped in a single transaction.
- Before executing, Cellar runs a re-validation: re-reads affected rows by PK to detect concurrent modifications. If any are detected, the user is prompted to merge, overwrite, or cancel.
- On success: grid refreshes, pending state clears, a toast confirms.
- On failure: full transaction rollback, error shown inline with the offending statement highlighted.

#### Foreign key navigation

- Cells holding FK values show a small link icon.
- Click to jump: opens a new tab with the referenced row pre-filtered.

### 6.6 Results, messages, plan, history (bottom panel)

Each query tab has a collapsible bottom panel with tabs:

- **Results** — the grid (when query returns rows)
- **Messages** — server notices, warnings, info
- **Plan** — execution plan visualization (Postgres `EXPLAIN ANALYZE`, SQL Server estimated/actual plans). Tree view with cost heatmap.
- **History** — recent queries from this tab/connection
- **Notices** — Postgres NOTICE/RAISE output, SQL Server `PRINT`, etc.

### 6.7 AI assistant

AI is a first-class feature, not an afterthought. It lives in the right panel and as inline completion in the editor.

#### Providers

- Anthropic (Claude models)
- OpenAI (GPT models)
- DeepSeek (V4 models)
- Ollama (any local model)
- Custom OpenAI-compatible endpoints (Together, Groq, etc.)

Providers are local and bring-your-own-credential. Keys are stored in `cellar-secrets`, and Cellar never proxies AI requests through a hosted service.

OpenAI supports two authentication modes:

- **Platform API key** — usage-based access through the Responses API. The Rust backend loads the key from `cellar-secrets`; it is never returned to the renderer.
- **ChatGPT sign-in** — subscription access through a local Codex app-server browser or device-code OAuth flow. Codex owns token storage and refresh in the OS keychain.

See `docs/architecture/adr/0002-openai-auth.md` for the trust boundary and runtime constraints.

DeepSeek uses a backend-only API key and the provider's OpenAI-compatible Chat
Completions API. Models are discovered live, and thinking mode is an explicit
provider setting. See `docs/architecture/adr/0003-deepseek-provider.md`.

#### Modes

1. **Chat panel** — right side, persistent across tabs but context-aware.
2. **Inline completion** — ghost text in the SQL editor. Triggered by typing or `⌘.`. Tab to accept, Esc to dismiss.
3. **Selection actions** — right-click on a table, column, or result cell to get AI actions: "Explain this table," "Write a query against this," "Ask about this value."

#### Context chips

The chat panel shows what context is being sent to the model as removable chips:

- Schema scope (`schema: public`)
- Tables in context (`table: orders`, `table: customers`)
- Active query (`query: revenue_by_country.sql`)
- Selection (`selection: lines 12-18`)
- Result rows (`result: 10 rows from last query`)

Users can add or remove chips manually. The exact payload sent to the provider is inspectable via a "View context" button — full transparency.

#### Generated SQL

When the model returns SQL:

- The SQL block renders with syntax highlighting.
- Buttons: `Insert into editor`, `Run`, `Explain`, `Copy`.
- Destructive statements (`DROP`, `DELETE` without WHERE, `TRUNCATE`, `UPDATE` without WHERE) are flagged with a warning and require explicit confirmation.
- Statements that would run against a `prod`-tagged connection require an extra confirmation.

#### Read-only mode

Default for the AI panel is **read-only**: generated queries can be displayed but `Run` is gated behind explicit user click. A toggle allows "auto-run read-only queries" for users who want a tighter loop.

#### Bottom action bar

Quick presets: `generate`, `explain`, `optimize`, `migrate`, `ask`. Each is a prompt template that takes the current context.

#### Conversation persistence

Conversations are saved per connection. Closing and reopening the panel resumes where you left off. History is browsable.

#### Cost transparency

Each message shows token count and estimated cost based on the provider's published rates. A cumulative session counter is shown in the panel header.

### 6.8 Settings

Settings are stored in `~/.cellar/settings.json`. Categories:

- **General** — theme (system / light / dark), font size, dense mode
- **Editor** — font family, tab size, format on save, autocomplete behaviour
- **Grid** — default page size, date format, NULL display, max cell preview length
- **AI** — provider, authentication mode, model, credential status, default context behaviour
- **Connections** — default timeout, default SSL mode
- **Keymap** — full keymap is configurable; presets for VS Code, DataGrip, Vim
- **Plugins** — installed plugins, enable/disable, settings
- **Telemetry** — off by default; opt-in with a self-hosted endpoint option

### 6.9 Keyboard

Cellar is keyboard-first. Every action has a shortcut. The full keymap is documented in `docs/keymap.md` and configurable in settings.

A command palette (`⌘K`) provides search-driven access to every action.

---

## 7. Non-functional requirements

### Performance

- Cold start: under 2 seconds on a modern laptop.
- Connection: under 1 second to a local DB, under 3 seconds to a remote DB on a good network.
- Schema introspection of a 500-table DB: under 3 seconds, cached after.
- Query of 1M rows: streams results progressively, first page visible within 500ms of server response.
- Grid scroll: 60fps with 1M+ rows.
- Memory: under 300MB resident for a typical session (one connection, a few open tabs).

### Reliability

- Crash recovery: restart restores open tabs, unsaved queries, and (with confirmation) pending edits.
- No data loss on app crash or kill — pending edits persist to local SQLite until committed or discarded.
- Connection drop is detected and surfaced. Auto-reconnect with exponential backoff, with user notification.

### Security

- Credentials never written to plain-text config files. Always in OS keychain or encrypted with a derived key.
- AI requests never include database credentials. Provider credentials stay in the OS keychain and outside the renderer where the provider supports a backend transport.
- AI requests are inspectable before sending.
- No telemetry without explicit opt-in.
- GPUI exposes no arbitrary shell or file capability to feature code. The legacy
  Tauri allowlist remains locked down until that client is removed.

### Cross-platform

- macOS 12+ (universal binary, signed and notarized)
- Windows 10+ (signed)
- Linux: AppImage, deb, rpm. Tested on Ubuntu 22.04+, Fedora 38+

---

## 8. UI specification

### 8.1 Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│ Cellar    [shop-eu (prod) ▸ shop_eu ▸ public]    [search]   [⚙]      │  ← title bar
├──────────┬──────────────────────────────────────────────┬────────────┤
│          │ Tab │ Tab │ Tab │ + │              [splits]  │            │
│ CONNS    ├──────────────────────────────────────────────┤            │
│          │                                              │ AI         │
│ filter   │           Editor or Grid                     │ ASSISTANT  │
│          │                                              │            │
│ ▼ conn 1 │                                              │ context    │
│   ▼ db   │                                              │ chips      │
│     ▼ pu │                                              │            │
│       t1 │                                              │ chat       │
│       t2 │                                              │            │
│       …  │                                              │ generated  │
│ ▶ conn 2 │                                              │ SQL block  │
│ ▶ conn 3 │                                              │            │
│          ├──────────────────────────────────────────────┤            │
│          │ Results │ Messages │ Plan │ History │ Notices│            │
│          ├──────────────────────────────────────────────┤            │
│          │                                              │            │
│          │           Results grid                       │            │
│          │                                              │            │
├──────────┴──────────────────────────────────────────────┴────────────┤
│ status bar: connection · engine · SSL · 4 pending · 10 rows · 84ms   │
└──────────────────────────────────────────────────────────────────────┘
```

The mockup in `docs/design/cellar-main.png` (and on claude.ai/design) is the canonical reference.

### 8.2 Design tokens

Defined in `packages/ui/src/tokens/`. Dark theme is default. Light theme available.

Core principles:

- Dense but breathable. Information density wins; whitespace earns its place.
- Monospace for SQL, data, identifiers. Sans-serif for chrome.
- No gradients. No rounded-everywhere. Functional.
- Single accent color (configurable). Default: a desaturated green for active state, amber for warnings, red for destructive/prod.

### 8.3 Accessibility

- All interactive elements reachable by keyboard.
- ARIA labels on icon-only buttons.
- Color is never the only signal: pending states use both color and an icon/dot.
- Configurable font size, line height.
- High-contrast theme variant.

---

## 9. Plugin API

The plugin system is foundational, not bolted on. v1.0 ships with three plugin types.

### 9.1 Plugin types

#### Drivers

A driver implements the `Driver` trait from `cellar-core`. First-party drivers live in `crates/cellar-drivers/`. Community drivers can be:

- **In-tree** (PR to the main repo)
- **External** (loaded at runtime via `cellar-plugin-host`)

Driver authoring guide: `docs/drivers/writing-a-driver.md`.

#### AI providers

An AI provider implements the `AiProvider` interface from `packages/ai`. First-party: Anthropic, OpenAI, DeepSeek, Ollama. Community providers register via the plugin SDK. A first-party provider may use typed Rust IPC when its credentials or supported authentication flow should not enter the renderer; OpenAI and DeepSeek use this boundary today.

#### Exporters

An exporter takes a result set and produces a file. First-party: CSV, TSV, JSON, SQL INSERTs. Community plugins can add Parquet, Avro, Markdown table, etc.

### 9.2 Loading

External plugins live in `~/.cellar/plugins/`. Each plugin is a folder with a `manifest.json` declaring type, version, entry point, and required capabilities.

v1.0 loading mechanism:

- **TS plugins** (AI providers, exporters): loaded as ES modules, run in the frontend renderer with the plugin SDK's restricted API surface.
- **Rust plugins** (drivers): out-of-process binaries communicating with `cellar-plugin-host` over a stable JSON-RPC protocol. Out-of-process is chosen over dylib loading for crash isolation and ABI stability.

### 9.3 SDK

`packages/plugin-sdk` exports:

- TypeScript interfaces for each plugin type
- A `manifest.json` schema
- Helpers for capability declaration and permission prompting
- A local dev mode that loads from a folder for plugin authoring

---

## 10. Build, release, distribution

### Build

- `pnpm dev` — runs the GPUI desktop client
- `pnpm build:native` — builds the optimized GPUI binary
- `pnpm dev:tauri` and `pnpm build` — keep the legacy parity client buildable during migration
- `cargo test` — runs Rust tests
- `pnpm test` — runs frontend tests
- `pnpm e2e` — runs Playwright end-to-end tests

### CI

GitHub Actions:

- `ci.yml` — lint, test on every PR. Matrix: macOS, Ubuntu, Windows.
- `test-matrix.yml` — integration tests against real DBs in containers (Postgres 14/15/16, MySQL 8, SQL Server 2022, SQLite). Runs nightly and on PRs touching drivers.
- `release.yml` — triggered by tag push. Builds signed artifacts for all platforms. Uploads to GitHub Releases.

### Release cadence

- Patch releases: as needed
- Minor releases: roughly every 4-6 weeks
- Pre-1.0 versions use `0.x.y`; the first stable release is `1.0.0`

### Distribution

- GitHub Releases (primary)
- Homebrew cask (macOS)
- Winget (Windows)
- Flatpak (Linux, post-1.0)
- AUR package (Linux, community-maintained)

Auto-updates use signed platform artifacts and retain selectable stable and beta
channels. The updater must verify the project signature before replacement.

---

## 11. Telemetry, privacy, data handling

- No telemetry by default. Period.
- Optional, opt-in telemetry can be enabled in settings. The telemetry endpoint is **user-configurable** — Cellar does not ship a default endpoint. If you want to collect your own usage data on your own infrastructure, you can.
- AI requests are sent only to the user-configured provider. Cellar does not proxy.
- No data leaves the machine unless the user explicitly:
  - Sends a query/schema to an AI provider, or
  - Enables telemetry with a configured endpoint
- Crash reports are local-only by default. A "send crash report" button is available with a preview of exactly what would be sent.

---

## 12. Roadmap

### v0.1.0 (alpha, ~day 30)

- Postgres driver
- Connection management with keychain storage
- Sidebar with schema tree
- SQL editor with basic autocomplete and execution
- Read-only data grid with virtualization, sort, filter
- Results panel

### v0.2.0 (~day 50)

- Cell, row, and column editing with pending changes
- Review & Commit flow with transactional SQL
- Foreign key navigation
- Query history
- Settings

### v0.3.0 (~day 65)

- SQL Server and Azure SQL drivers
- MySQL and SQLite drivers
- Execution plan visualization (Postgres at minimum)
- Cross-engine SQL formatting and dialect handling

### v0.4.0 (~day 80)

- AI panel: chat, context chips, generated SQL
- AI inline completion in the editor
- Anthropic and OpenAI providers
- Ollama provider

### v0.5.0 (~day 90)

- Plugin SDK and runtime
- First-party exporters (CSV, JSON, SQL)
- Auto-updater
- Polish, performance, cross-platform packaging

### v1.0.0 (~day 100)

- Signed, notarized binaries for macOS, Windows, Linux
- Documentation site
- Plugin authoring docs
- Public launch

### Post-1.0 (not in scope for this spec)

- More drivers (ClickHouse, DuckDB, MariaDB, CockroachDB, Snowflake)
- ER diagram view
- Saved queries, parameterized queries, query templates
- Snippet library, shared via plugins
- Team sync (encrypted, optional, self-hostable)
- WASM-based plugin runtime
- Web build (subset of features) for read-only browsing

---

## 13. Open questions

These are tracked in `docs/architecture/open-questions.md` and revisited regularly. Listed here for awareness:

- **Grid library:** TanStack Table is the current pick. AG Grid Community is more capable but has license restrictions on some features. Revisit if TanStack hits limits.
- **Plugin distribution:** Should there be a curated index? If yes, who curates? Start without one.
- **Multiple result sets per query (SQL Server stored procs):** UX is undecided. Tabs? Stacked? Configurable.
- **Schema diff and migration generation:** Often-requested feature. Likely post-1.0 unless it lands cleanly within scope.
- **Multi-statement editor execution and result correlation:** Each statement gets its own results tab — but how to manage many results without overwhelming the UI?

---

## 14. Contributing

Cellar is community-led. Contributions are welcome from day one.

- All issues and discussion happen on GitHub.
- See `CONTRIBUTING.md` for setup, code style, and PR guidelines.
- See `docs/drivers/writing-a-driver.md` to add support for a new database.
- See `docs/plugin-authoring.md` to write an AI provider or exporter.
- Good first issues are labeled `good first issue`.

Architectural changes should be proposed via an ADR (Architecture Decision Record) in `docs/architecture/adr/`. ADRs are short, dated, and capture context, decision, and consequences.

---

## 15. Glossary

- **Connection** — a configured database endpoint (host, port, credentials, engine).
- **Driver** — Rust code that implements the `Driver` trait for a specific engine.
- **Pending change** — an unsaved edit (insert, update, delete) held in the grid.
- **Commit** — applying pending changes as a transactional SQL script.
- **Context chip** — a piece of context (schema, table, query, selection) attached to an AI request.
- **Plugin** — an external driver, AI provider, or exporter loaded at runtime.

---

*End of spec. This is a living document. Significant changes should be proposed as a PR with an ADR.*
