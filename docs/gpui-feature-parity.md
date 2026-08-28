# GPUI feature parity

This checklist is the removal gate for `apps/desktop` and its Tauri entrypoint.
An item is complete only when the GPUI implementation uses the real shared
runtime and has a runnable check or recorded manual verification.

Legend: `[x]` verified in GPUI; `[ ]` not yet at parity.

## Foundation

- [x] Clean migration branch based on `origin/main`.
- [x] Shared Rust connection/runtime state used by both desktop clients.
- [x] Native GPUI window loads real saved connection metadata.
- [x] Persist and restore validated window bounds, sidebar width, active
  connection, grid column order/widths, sorts, and filters in a versioned local
  session file with restricted permissions and redacted SQL secrets.
- [x] macOS debug and optimized release builds.
- [x] macOS, Linux, and Windows optimized-build CI paths.
- [ ] Linux and Windows debug and release builds.
- [ ] Signed bundles, updater, and release workflow.

## Connections and navigation

- [x] Dense connection list with engine and production indicators.
- [x] Connect, disconnect, reconnect, and refresh through the shared runtime.
- [x] Add, edit, test, and explicitly confirmed delete connections.
- [x] Native connection editing preserves engine, SSL mode, environment tag,
  application name, and validated accent colour metadata.
- [x] Duplicate connection metadata into a new unsaved connection; keychain
  passwords are intentionally not copied.
- [x] DataGrip connection import scans local projects off the UI thread, shows
  unsupported/skipped entries, supports deliberate existing-ID overwrite,
  editable database targets, and optional user-entered keychain passwords while
  never reading passwords from DataGrip.
- [x] Masked passwords persist through `cellar-secrets`; credentials remain
  outside connection config and logs.
- [x] Live schema introspection and explicit refresh.
- [x] Tree hierarchy matches Tauri's database, schema, tables/views group, and
  relation rows; clicking a table or view opens its data tab.
- [x] Connection/folder filtering with Cmd/Ctrl+F and automatic expansion of
  matching folders.
- [x] Searchable, persisted schema visibility controls with classic empty-schema
  auto-hiding and visible/total counts.
- [x] Live command-palette catalogue search across tables, views, and columns.
- [x] View rows match the classic sidebar behaviour (openable data rows without a non-canonical expandable column subtree).
- [x] Sidebar width is drag-resizable with the classic 200–600 px bounds.
- [x] Schema-tree keyboard navigation: Tab focus, Up/Down traversal,
  Left/Right collapse and expand, and Enter/Space activation.

## Workspace and SQL

- [x] Open, close, reorder, and keep native tab/editor/grid state alive.
- [x] Horizontal and vertical workspace splits keep independent pane focus and
  active tabs, use a draggable native divider, collapse when a pane empties,
  and persist pane membership across launches.
- [x] Restore ordered table/query tabs, pin state, active tab, and query editor
  contents across app launches; obvious SQL credentials are redacted before
  persistence.
- [x] Table, SQL, and find-usages workspaces use the shared native runtime.
- [x] Schema-compare and ER-diagram tabs use live shared metadata and restore across sessions.
- [x] Native SQL editor with SQL syntax highlighting and persistent per-tab
  editor state.
- [x] Postgres named/positional parameters are tokenized through `cellar-sql`,
  collected in a native panel, type-inferred where unambiguous, validated, and
  bound through the driver protocol without interpolation.
- [x] Run-current-statement and run-all match the classic editor behaviour.
- [ ] Current-statement line-band, error-line background, and bracket-match
  decorations match CodeMirror. GPUI Component 0.5.1 exposes syntax and
  diagnostic styling but not arbitrary input highlights; upstream issue #1989
  tracks the missing API. GPUI currently uses the canonical error range as a
  native diagnostic underline rather than silently dropping it.
- [x] SQL formatting remains visibly disabled, matching the classic editor's current unimplemented control.
- [x] Schema-aware completion matches classic relation, schema, column, alias,
  keyword, and snippet suggestions.
- [ ] Broader dialect-specific editor handling.
- [x] Bounded query pages append progressively to the native grid.
- [x] In-flight query cancellation, including cancellation when closing a tab.
- [x] Per-run completion messages, affected-row counts, timing, and captured
  database notices.
- [x] Results, messages, plans, history, and notices render in the shared bottom
  output panel rather than duplicating content below the SQL editor.
- [x] Retained local query history uses the shared redacting SQLite store and
  can reopen SQL in a native query tab.
- [x] Safe Postgres EXPLAIN estimates render from the shared typed plan tree.
- [x] EXPLAIN ANALYZE requires a second explicit click that states it executes
  the SQL before loading the typed plan tree.
- [x] Query row-count, running, completion, timing, and error state.
- [x] Native status bar shows active connection state and query-history count;
  query tabs show live receive/completion state.

## Data grid

- [x] Row and column virtualization with small overscan.
- [x] Frozen first column.
- [x] Drag column resizing with 64–600 px safety bounds.
- [x] Drag column reorder remaps row values, widths, selection, sorting, and
  pending edits by index.
- [x] Type-aware display, SQL NULL, and duplicate column names by index.
- [x] Mouse and keyboard selection/navigation implemented.
- [x] Selection/navigation verified within one frame in an optimized build.
- [x] Deterministically ordered, bounded server paging with total-row status.
- [x] Click-to-cycle server sorting with visible direction state.
- [x] Bounded server-side quick filtering uses `contains` for textual columns
  and typed equality elsewhere.
- [x] Advanced server filters can combine multiple typed clauses with the full
  supported operator set; values remain driver-bound.
- [x] Filter presets snapshot and restore quick filters, advanced filters, and sorting per table, and persist across GPUI sessions.
- [x] Text inline editor remains mounted outside virtualized cells; pending
  updates and SQL NULL values survive scrolling.
- [x] Cross-platform copy and bounded TSV paste from the selected cell.
- [x] Inline edits validate integer, numeric, boolean, and ISO-date values;
  boolean cells also have a native toggle action before review.
- [x] Pending inserts append editable native rows and use typed review,
  transactional commit, and revert.
- [x] Pending row deletes use the same typed review, transactional commit, and
  revert flow as updates.
- [x] Pending updates use typed `cellar-diff` review, transactional commit, and
  revert through the shared runtime.
- [x] CSV, TSV, JSON, and SQL export use the native save dialog and a shared
  streaming Rust formatter/atomic writer off the UI thread; duplicate columns
  and SQL NULL remain lossless.
- [x] Native CSV/TSV/semicolon import parses off-thread, preserves NULL versus
  quoted empty strings, supports column mapping and update/insert/upsert roles,
  validates keys/required fields, previews at most 25 statements, and commits
  the full bounded request transactionally through the shared runtime.

## Remaining product surfaces

- [x] Native command palette opens from the title-bar search and Cmd/Ctrl+K,
  filters live tabs/connections/actions, and runs those real actions.
- [x] Connection, folder, database, schema, relation, tab, grid, and result
  context menus match the classic action set; Find Usages searches the shared
  catalog cache and opens matching objects.
- [x] Remaining confirmations, empty states, and settings match the classic
  catalogue, actions, disabled states, dimensions, spacing, icons, and local
  persistence behaviour.
- [x] AI panel, inspectable context, provider settings, ChatGPT sign-in,
  conversation history, and safe generated-SQL actions without exposing
  credentials. The classic AI request path is non-streaming and has no request
  cancellation control, so GPUI matches that behaviour.
- [x] Schema comparison and migration review with live/snapshot sources, selectable statements, confirmation gates, and transactional apply.
- [x] DataGrip connection import.
- [x] Broader setup transfer with file/paste review, selectable connection and
  preference sections, conflict handling, and add/replace/skip results.
- [x] Theme, bundled fonts, native application menus, classic shortcuts, and
  external links.
- [ ] Semantic accessibility for custom GPUI controls.
- [x] Custom native controls participate in tab order and use GPUI's built-in
  Enter/Space click activation; schema-tree rows retain arrow-key navigation.
- [x] Native title bar uses Tauri geometry and always toggles maximise/restore
  on double-click, matching classic Cellar even when macOS disables the native
  title-bar action.

Latest macOS launch capture verified the shell renders without a crash. GPUI's
current accessibility tree exposed only the native window buttons and one text
node; semantic accessibility for custom controls remains open.

## Performance gates (optimized builds)

- [x] 60 fps scrolling at 10,000 rows by 500 columns.
- [x] Rendered cells are bounded by viewport plus overscan.
- [x] No complete result materialization at the UI boundary.
- [x] Warm first usable window under one second on the development Mac.
- [x] Idle RSS under 150 MiB.
- [x] Selection and keyboard navigation paint within one frame.
- [x] Query pages paint before execution completes: streaming-driver tests emit
  bounded pages before completion, and the GPUI model remains in `Running`
  while each appended page updates row count and schedules a repaint.
- [x] Repeatable optimized startup/RSS and production-grid regression checks.

Latest optimized macOS measurement (`pnpm perf:native:startup`) with the native
SQL editor, restored-session support, and DataGrip import: 271 ms to a
detectable window and 82,512 KiB idle RSS. The first launch immediately after
the release link measured 3,402 ms; cold-cache
startup remains open until that one-time GPUI/font-cache cost is isolated or
removed.

The optimized production-grid check (`pnpm perf:native:grid`) drives selection,
vertical reveal, horizontal reveal, layout, and paint for 240 frames against
10,000 rows by 500 columns. It measured 119.6 mean fps and a 9.55 ms p95 frame
interval on the development Mac.

## Removal gate

- [x] Full Rust, frontend, release-build, and performance suites pass.
- [ ] Automated review has no unresolved actionable findings.
- [ ] Tauri/React removal produces no supported-workflow regression.
