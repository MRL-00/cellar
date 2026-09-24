# Changelog

## 1.0.9

### Bug fixes

- **Grid and filter menus stay usable** — columns scroll normally instead of
  remaining frozen, and the filter-preset menu stays within the window without
  stretching wider than intended.

## 1.0.8

### Features

- **Syntax-highlighted JSON cells** — JSON values now use syntax highlighting
  both in the grid and in the expanded viewer, making keys, strings, numbers,
  and other values easier to scan.

### Bug fixes

- **Modal dialogs block background clicks** — clicking controls behind dialogs
  and menus no longer activates them while an overlay is open.

## 1.0.7

### Bug fixes

- **Row copy works straight from the keyboard** — opening a table focuses the
  grid and the Edit menu's Copy item copies the grid selection, so Cmd/Ctrl+C
  copies selected rows as JSON without clicking into the grid first.
- **Columns fit their data again** — columns size to their header, type, and
  loaded values, keep fitting as more rows load, and widths you resize survive
  paging, sorting, and reloads.
- **Clean frozen column when scrolling sideways** — the row numbers and first
  column stay pinned above the columns scrolling under them, instead of the two
  overlapping on selected rows.

## 1.0.6

### Features

- **Table structure in the sidebar** — expand a table row to see its columns,
  keys, indexes, and defaults without opening the table. Folders open by
  default, remember their collapsed state per table, and toggle with Left/Right
  without opening the table itself.
- **JSON row copy and paste** — copying selected rows writes JSON objects keyed
  by column name, and pasting that JSON back into the grid maps values by name,
  so column order and absent keys are handled for you. Plain text still pastes
  as tab-separated values, and the context menu keeps its CSV, TSV, JSON, and
  SQL INSERT copies.

### Bug fixes

- **Restored table tabs load again** — tabs restored on launch no longer stop at
  "Table metadata is unavailable" when they open before the connection
  finishes; they load as soon as schema metadata is ready.
- **Last grid rows stay visible** — the grid now ends above the horizontal
  scrollbar instead of hiding the final rows behind it.

## 1.0.5

### Features

- **Cell range selection** — drag across grid cells or Shift-click to extend a
  rectangular selection, then copy it as tab-separated values for pasting into
  spreadsheets and other table tools.
- **Visible grid scrollbars** — use the always-visible horizontal and vertical
  scrollbars to move through wide tables and large result sets.

### Bug fixes

- **Correct grid scrolling** — Shift+wheel now scrolls horizontally without
  moving rows, while ordinary wheel and trackpad gestures continue to scroll
  vertically.
- **Reliable connection editor focus** — connection fields keep keyboard focus
  across their full input area, and Tab navigation stays inside the dialog.

## 1.0.4

### Features

- **TablePlus connection import** — discover and import saved TablePlus
  connections in one step, preserve groups as sidebar folders, map SSL modes,
  and clearly identify SSH-tunnelled connections that are not yet supported.

### Bug fixes

- **Reliable TablePlus discovery** — scan results now reach the import dialog,
  Windows uses the correct TablePlus data directory, and a folder's own
  connection is retained when it shares the folder name.
- **Unsigned MySQL schema browsing** — tables with unsigned integer metadata
  no longer fail during schema introspection.

## 1.0.3

### Bug fixes

- **Aurora MySQL schema browsing** — connections to Aurora MySQL no longer
  fail during schema introspection when textual metadata is reported as binary.

## 1.0.2

### Bug fixes

- **Hostname field stays visible** — the host input in the connection
  editor no longer collapses, so the server address and port are always
  editable.

## 1.0.1

### Bug fixes

- **Scrolling stays inside dialogs** — wheel and trackpad gestures on a
  modal backdrop, such as the command palette or confirmation dialogs,
  no longer scroll the grid and panels behind them.
- **Multiline cells stay on one line** — text values containing line
  breaks, such as pretty-printed JSON stored in nvarchar columns, now
  render as a single truncated line with ⏎ markers instead of
  overlapping neighbouring rows. Editing and copying keep the real
  line breaks.
- **Visible table separators** — borders around the tab strip, column
  headers, and table footer are visible again on dark backgrounds
  instead of blending into the panel.
- **Cleaner updater packages** — update tarballs no longer include
  stray macOS metadata files.

## 1.0.0

### Features

- **New GPUI desktop client** — Cellar is now a native GPUI app. The data
  grid stays fast and responsive on wide tables and large result sets with
  row and column virtualization, keyboard-first navigation, and a denser,
  work-focused UI.
- **Unified table footer** — export, CSV import, row actions, pending-edit
  state, refresh, and pagination now live on a single icon bar under the
  grid, with tooltips for every action.
- **Row multi-select** — select whole rows from the gutter, extend with
  Shift, and toggle individual rows with Cmd; copy and delete act on the
  full selection.
- **Better table filters** — column pickers for quick filter, filter, and
  order-by are real dropdown menus sorted alphabetically, and the bar's
  text and controls scale with the UI scale setting.

### Bug fixes

- **Reliable reloads** — sorting, filtering, or refreshing a table no
  longer flashes a blank panel or discards edits made while data reloads;
  pending edits are protected and editing controls freeze until the reload
  completes.
- **Safer bulk deletes** — deleting a mixed selection no longer unmarks
  rows that were already marked for deletion.
- **Scrolling behaves** — the grid no longer scrolls sideways on vertical
  trackpad gestures, horizontal scroll follows wheel intent, and dropdown
  menus stay open while their lists scroll.
- **Clearer connection errors** — connection failures surface in a modal
  instead of being swallowed.
- **Mac polish** — the Dock shows the Cellar name and icon, Cmd+K opens the
  command palette reliably, connection names expand correctly, and compact
  inputs no longer carry extra padding.

## 0.3.9

### Bug fixes

- **Table filters survive tab switches** — filter chips and the quick-filter
  text stay in place when you move between tables, including values you were
  still typing.
- **Tables stay loaded when you switch away** — visited table panes remain
  mounted, so switching back is instant and no longer reloads rows or resets
  the grid.

## 0.3.8

### Features

- **OpenAI and ChatGPT authentication** — connect the AI assistant with an
  OpenAI API key or sign in with a ChatGPT subscription. Requests go directly
  to OpenAI without passing through a hosted Cellar service.
- **DeepSeek support** — use DeepSeek models with credentials stored in the OS
  keychain, automatic model discovery, and an optional thinking mode for more
  complex requests.
- **Inline AI model switching** — change models directly from the AI composer
  without returning to Settings.
- **Type-aware table filters** — boolean columns now use a true/false selector,
  while date and time columns use the shared calendar and time picker.
- **Improved result exports** — choose the destination and filename using the
  native save dialog. Export controls are larger, and query results
  automatically open at a useful panel height.

### Bug fixes

- **Safer export writes** — exported files are written atomically to reduce the
  risk of incomplete files, and failures are now reported in the Messages
  panel.
- **More reliable AI sign-in** — ChatGPT authentication opens in the system
  browser and handles provider configuration more consistently.
- **Clearer DeepSeek failures** — provider error details and HTTP status codes
  are preserved instead of being replaced with generic messages.

## 0.3.7

### Features

- **Enter to send in AI chat** — press Enter to send a message; Shift+Enter
  inserts a newline.
- **SQL Server AI query answers** — AI-generated read-only SQL can now run on
  SQL Server and Azure SQL inside a transaction that always rolls back.
- **Richer AI schema context** — the assistant resolves relevant tables,
  relationships, and named entities before answering, with clearer loading
  states while it prepares.

### Bug fixes

- **Filter dropdown scrolling** — column and preset filter menus stay open
  while you scroll their lists; only scrolling the grid dismisses them.
- **SQL Server read-only sessions** — failed read-only AI runs no longer leave
  the connection in a contaminated transaction state.
- **macOS app icon padding** — the Dock icon mark sits further from the tile
  edges for better optical sizing.

## 0.3.6

### Features

- **Azure Cosmos DB** — connect with an account endpoint and primary key, browse
  databases and containers in the sidebar, and open documents in the grid.
  Read-only for now; Cosmos SQL in the editor lands later. Nested document
  fields use the existing JSON cell viewer (summary in the grid, expandable
  tree with copy). Grid filters run server-side; sorts are applied locally to
  the current page. `contains` on a JSON column searches inside the document
  blob.
- **Folder color accents** — tag sidebar folders from the context menu with a
  left-edge color marker so related connections are easier to spot.

## 0.3.5

### Features

- **Selective setup exports** — choose exactly which saved connections to
  include, use Select all or Unselect all for quick changes, and export only
  the table layouts belonging to those connections.
- **Remembered table sorts** — restore each table's last column sort when it is
  reopened. The behavior is enabled by default and can be changed in Data grid
  settings.
- **Generate GUID values** — right-click UUID, GUID, or `uniqueidentifier` cells
  to generate a new value through the normal pending-edit workflow.
- **Copy cells from the context menu** — table cells now offer the same Copy
  cell action as query results.

### Bug fixes

- **SQL Server grid commits** — transaction control and database switching now
  run as raw batches, fixing failed commits caused by mismatched transaction
  counts.
- **Boolean cell editing** — SQL Server `bit` columns now use a TRUE, FALSE, and
  optional NULL selector instead of free text.
- **Editor cleanup after commits** — inline cell editors close when refreshed
  after a commit and no longer reopen from the same double-click.
- **Postgres bit strings** — Postgres `bit` and `bit(n)` values keep their text
  editor instead of being mistaken for SQL Server booleans.
- **Date picker placement** — date and datetime editors now flip above cells or
  clamp horizontally when needed instead of being clipped by grid controls.
- **Resizable Messages columns** — message metadata no longer overlaps, and
  every column can be resized from its header.
- **Export and sort edge cases** — empty connection selections no longer leak
  table layouts into setup exports, saved sorts apply as soon as remembering is
  enabled, and temporarily missing columns no longer erase persisted sorts.
- **Production website styles** — Tailwind is now available during Nixpacks
  builds, preventing missing production styles.
- **Social link previews** — links now use a purpose-built Cellar social card
  with explicit Open Graph and X metadata, dimensions, format, and alt text.

## 0.3.4

### Features

- **SQLite support** — connect to local SQLite databases, browse schemas and
  tables, and run queries through the new first-party driver.
- **Hosted database providers** — Supabase, Neon, and PlanetScale now have
  dedicated connection options, branding, and the correct SQL dialect support.
- **Convex browsing** — connect to local, self-hosted, or cloud Convex
  deployments and browse tables through the streaming export API.
- **SQL Server grid commits** — review and commit grid edits and CSV imports on
  SQL Server and Azure SQL with dialect-aware previews and transactional safety
  checks.
- **AI query answers** — safe, read-only SQL generated in the AI panel can be
  inserted into the editor or run directly, with results shown inline.
- **New Cellar identity** — refreshed desktop and platform icons, in-app marks,
  browser favicons, and a downloadable logo pack with monochrome variants.
- **Marketing site redesign** — rebuilt the site with React, TypeScript,
  Tailwind CSS, responsive motion and parallax effects, refreshed screenshots,
  and separate Apple Silicon and Intel downloads.

### Bug fixes

- **Dialect-aware SQL** — query history, commit previews, nullable-key matching,
  and upserts now use the connected database's SQL dialect.
- **AI workflow safety** — read-only execution is guarded by engine support and
  protected from stale or overlapping assistant runs.
- **Site accessibility** — added keyboard navigation, visible focus states,
  reduced-motion support, and improved responsive behavior.

## 0.3.3

### Features

- **Saved filter presets** — save the current table filters as a named preset
  from the grid toolbar and re-apply them later.
- **Order by control** — sort results straight from a new Order By control in
  the filter bar.
- **More filter operators** — comparison and pattern operators (evaluated
  server-side), themed operator dropdowns, and fixes for filtering GUID
  columns.
- **Cmd/Ctrl+F** — now focuses the sidebar filter input.
- **Release notes catch-up** — GitHub releases publish the changelog section
  for each version, and the in-app Updates panel shows the notes for every
  version since the one you have installed.

### Bug fixes

- **Data grid** — softer contrast, content-fit column widths (remeasured on
  density change), duplicate row-count indicators removed, and literal
  wildcard characters in filters are now escaped.
- **Sidebar** — connection rows no longer show visual markers, the connected
  status dot is green, all tree labels render at the configured font size,
  and connections can be moved to an existing folder from the context menu
  again (without the per-folder clutter).
- **Desktop UI** — larger, more consistent font sizes across the bottom
  panels, sidebar, and status bar; light-theme overrides for insert tokens.

### Internal

- Split oversized source files to enforce the 800-line file limit.

## 0.3.2

### Features

- **Bundled interface & editor fonts** — Geist, Inter, JetBrains Mono, and
  Roboto now ship with the app (with their OFL licenses), so the font pickers
  work offline and render consistently across machines.
- **Font settings applied everywhere** — the interface and editor fonts chosen
  in Settings are now wired through the whole UI instead of just being stored.
- **Update toast** — a bottom-right "Update available" toast appears on startup
  when a new version is found; its Update button opens Settings → Updates.
- **In-app release notes** — the Updates panel shows a "What's new" section with
  the pending version's release notes (falling back to the bundled changelog).

### Bug fixes

- **Sidebar tree** — unified the tree label font size and weight, and matched
  folder/database labels to the base font size.
- **Pane separators** — resizable pane separators are now visible on the dark
  theme.
- **Grid font** — preserved the mono font features in the data grid.

## 0.3.1

### Features

- **New default theme** — neutral default palette, SF Pro interface font, and
  1px icons, with the saved interface/editor font now respected throughout.
- **Font pickers in Settings** — dropdowns to choose the interface and editor
  fonts.
- **Reliable table row counts** — total row count shown without blocking the
  table load.
- **Cleaner sidebar** — the dashed "New connection" button only appears when
  there are no connections.
- **Wired-up About screen** — footer links and attribution now work.
- **Marketing site redesign** — cinematic hero, fixed social link previews, and
  improved SEO.

### Bug fixes

- **Quick filter** — clears immediately instead of waiting on the debounce, and
  stays smooth on large tables (fixed a notices leak).
- **Table caching** — `include_total` is now part of the browse cache key, so
  row-count results aren't served stale.
- **Settings** — live version number, separator dot hidden until the version
  loads, and a clearer General-tab placeholder.
- **Import** — vertically centered the mode selection dots.
- **Theme polish** — accent color stays visible everywhere, grid/data rows track
  the density font-size token, and the active bottom-panel tab is visible.

### Internal

- Release workflow stamps the release version into `tauri.conf.json` at build
  time from the pushed tag.
