# Top 5 — Implementation-Ready Breakdowns

Companion to [improvement-plan.md](improvement-plan.md). Each item lists the files to touch, the contract changes, the work, and acceptance criteria. Grounded in the actual code as of 2026-06-09.

Key architectural facts discovered:
- Driver contract: `Driver` trait in `crates/cellar-core/src/driver.rs:109` (`connect`/`introspect`/`execute_query`/`explain_query`). `browse_table` + `commit_table_changes` are driver free-functions, **not** trait methods.
- Query shape: `Query { sql, max_rows: Option<u32>, database }` (`crates/cellar-core/src/query.rs:9`). `QueryResult` already has `rows_affected: Option<u64>` and `truncated: bool` fields — partially plumbed.
- IPC commands registered via `collect_commands!` in `apps/desktop/src-tauri/src/commands/mod.rs:21`; generated TS lands in `packages/ipc/src/generated.ts`.
- `run_query` command (`commands/query.rs:12`) already accepts `max_rows: Option<u32>`; registry dispatch in `state.rs:210`.
- Settings are **frontend-only**: `localStorage` key `cellar.settings.v1`, type has just 6 fields (`theme/density/accent/fontSizePx/interfaceFont/monoFont`) in `apps/desktop/src/lib/settings.tsx:31`. No Rust/IPC involved.

---

## 1. Kill the 500-row cap → real pagination + virtualization

**Problem:** `DEFAULT_MAX_ROWS = 500` (`postgres/src/query.rs:16`) and `MAX_TABLE_BROWSE_ROWS = 500` (table_browse) silently truncate. `truncated` is plumbed to `QueryResult` but there's no way to fetch page 2.

**Contract changes (`crates/cellar-core/src/query.rs`):**
- Add `offset: Option<u32>` to `Query` (table browse already has `offset`).
- Add `total_rows: Option<u64>` to `QueryResult` for "showing N of M" (best-effort; `None` when unknown).

**Backend work:**
- `postgres/src/query.rs:execute_query` — honor `query.offset`; raise default page size to a setting-driven value (e.g. 1000) and support explicit "fetch next page". Keep the stream-and-cap approach (already good — it doesn't materialize the whole server result).
- `postgres/src/table_browse.rs` — already does `LIMIT $1 OFFSET $2`; raise the hard `MAX_TABLE_BROWSE_ROWS` cap to a page-size + allow paging via `offset`. Add an optional `SELECT count(*)` for `total_rows` (gated behind a setting — expensive on big tables).
- Mirror in `sqlserver/src/table_browse.rs` (already uses `OFFSET n ROWS FETCH NEXT m`).

**Frontend work:**
- `apps/desktop/src/components/BottomPanel.tsx` — footer "showing N of M · [Load more] [Fetch all]"; wire infinite-scroll page fetches into the grid.
- `@cellar/data-grid` — confirm it's virtualized for the larger row counts (windowed rendering). If TanStack Virtual isn't wired, add it.
- `commands/query.rs` — pass `offset` through; add a `fetch_query_page` command or reuse `run_query` with offset.

**Acceptance:** Browse a 1M-row table, scroll smoothly, footer shows true total, "load more" pages without re-running from row 0. No silent 500 truncation; truncation only when the user-set cap is hit, and it's clearly labeled.

---

## 2. Query cancellation

**Problem:** No cancel anywhere. A runaway query blocks the connection's pool slot with no escape.

**Contract changes:**
- Add `query_id: Option<String>` to `Query` (host-generated UUID per execution).
- Add trait method to `Driver` (`driver.rs:109`): `async fn cancel_query(&self, conn: &dyn Connection, query_id: &str) -> CellarResult<()>;` with a default impl returning `CellarError::unsupported` so non-Postgres drivers compile.

**Backend work (Postgres):**
- On `execute_query`, capture the backend PID via `pg_backend_pid()` on the acquired connection (or use a dedicated connection so the PID is knowable), store `query_id → (pid, database)` in a registry on `PgConnection`.
- `cancel_query` opens a **second** pool connection and runs `SELECT pg_cancel_backend($1)`.
- Clean up the registry entry on completion.
- Registry/state: `apps/desktop/src-tauri/src/state.rs` holds `ConnectionRegistry`; add an in-flight-query map keyed by `query_id`.

**IPC + frontend:**
- New command `cancel_query(connection_id, query_id)` in `commands/query.rs`, registered in `commands/mod.rs`.
- `run_query` returns/echoes the `query_id`; UI shows a "Cancel" button in the editor toolbar / bottom panel while a query is running; `⌘.` shortcut.

**Acceptance:** Run `SELECT pg_sleep(60)`, hit Cancel, query aborts within ~1s with a clean "cancelled" message, pool slot freed, connection still usable.

---

## 3. Persist settings + make them take effect

**Problem:** Almost the entire Settings modal is `StaticSegment`/`readOnly` with no `onChange`. The real settings store (`settings.tsx`) only models 6 appearance fields. Editor/Grid/General/History/Keymap panels save nothing and affect nothing.

**Work (frontend-only — no Rust needed):**
- Expand `Settings` type in `apps/desktop/src/lib/settings.tsx:31` with the panels' real fields:
  - Editor: `tabSize`, `softWrap`, `lineNumbers`, `bracketMatching`, `keywordCase`, `formatOnSave`, `statementRunMode`, `selectStarLimit`.
  - Grid: `rowHeight`, `nullDisplay`, `numberAlign`, `stripeRows`, `stickyPrimaryKey`, `truncateCells`, `defaultPageSize`.
  - General: `startupMode`, `confirmBeforeQuit`, `defaultSchemaSearchPath`.
- Extend `DEFAULTS`, `sanitize`, and `load`/persist (already merges `{...DEFAULTS, ...parsed}` so it's forward-compatible).
- Replace `StaticSegment`/`readOnly` controls in `settingsWorkspacePanels.tsx` (lines ~147–262), `settingsDataPanels.tsx`, with live `useSettings().set(...)` bindings.
- Make settings **take effect**: pass editor settings into `@cellar/sql-editor` (tab size, wrap, line numbers, bracket matching, run mode), grid settings into `@cellar/data-grid` (row height, NULL display, alignment, stripes, page size — ties into #1), and the `selectStarLimit`/page-size into the browse/query path.
- Fix the lying placeholders: replace hardcoded stats ("23,418 queries · 14.2 MB") in `settingsDataPanels.tsx:93` / `settingsSystemPanels.tsx` with real counts from the history store, or hide the banners until wired.
- Window toggles (`settingsWorkspacePanels.tsx:132`) — wire to Tauri window APIs or remove.

**Acceptance:** Change tab size / NULL display / page size; editor and grid reflect it immediately and after restart. No `readOnly` control in the modal that pretends to be editable. No fabricated stats.

---

## 4. Result export (CSV / TSV / JSON / SQL INSERT)

**Problem:** "Export not implemented yet" button (`BottomPanel.tsx:148`); no grid copy-as either. SPEC §9.1 promises CSV/TSV/JSON/SQL first-party exporters.

**Work (frontend-first; the data is already in the grid):**
- New module `apps/desktop/src/lib/export.ts` — pure functions `toCsv/toTsv/toJson/toSqlInserts(columns, rows, opts)`. Reuse `ColumnMeta` typing; respect NULL handling and quoting (CSV RFC 4180; SQL identifier/value escaping mirroring `cellar-diff`'s literal escaping).
- Wire the `BottomPanel.tsx:148` Export button → format menu → Tauri `save` dialog (`tauri-plugin-dialog`) + `fs` write. Add `tauri-plugin-dialog`/`fs` to `tauri.conf.json` capabilities if absent.
- Grid context menu (`@cellar/data-grid`): "Copy as TSV/CSV", "Copy row(s) as INSERT" → clipboard. Multi-row selection copy.
- Honor the active filters/sort and the **full** result, not just the visible page (depends on #1's fetch-all).

**Acceptance:** Run a query, Export → CSV/JSON/SQL writes a correct file; right-click rows → copy as INSERT yields runnable SQL; large exports stream rather than freeze the UI.

---

## 5. Schema-aware autocomplete in the SQL editor

**Problem:** The CodeMirror editor exists but has no completion from live schema. This is the #1 felt gap vs DataGrip. Introspection data (databases→schemas→tables→columns, with FKs) is already available in the Zustand connection store.

**Work (frontend, in `@cellar/sql-editor` + desktop wiring):**
- Add a CodeMirror `autocompletion` source fed by the current tab's connection schema (from the connections store, already populated by `introspect`).
- Context-awareness (start simple, iterate):
  - After `FROM`/`JOIN`/`UPDATE`/`INTO` → suggest `schema.table` (and bare table for the search-path schema).
  - After `SELECT`/`WHERE`/`ON`/`,` → suggest columns, preferring tables referenced earlier in the statement (parse the `FROM`/`JOIN` clause for table aliases).
  - Keywords + functions as a lower-priority tier.
- Use the tab's `connection_id`/`database` to scope suggestions to the right schema set.
- Snippet completions (`sel`, `ins`, `upd`, `del`, `jln`) per SPEC §6.4 as a quick add-on.
- Reuse FK metadata for future "join suggestion" but not required for v1.

**Acceptance:** Type `SELECT * FROM ` and get table suggestions from the connected DB; type `SELECT ` after a FROM and get that table's columns; suggestions update when the schema is refreshed; no suggestions leak across connections.

---

## Recommended build order for the loop

1. **#3 settings** (frontend-only, unblocks page-size for #1, immediate "feels finished" win).
2. **#1 pagination** (core; depends on grid virtualization + page-size setting).
3. **#4 export** (depends on #1's fetch-all for full-result export; otherwise independent).
4. **#2 cancellation** (backend trait change; isolated).
5. **#5 autocomplete** (frontend; independent; high perceived value).

Each is independently shippable. #1 and #4 share the "fetch all rows" plumbing, so sequence them together.
