# Changelog

## 0.3.5

### Features

- **Selective setup exports** — choose which saved connections to include in a
  setup bundle, with table layouts filtered to the selected connections.
- **Remembered table sorts** — optionally restore each table's last column sort
  when it is reopened; enabled by default in Data grid settings.
- **Generate GUID values** — right-click UUID, GUID, or `uniqueidentifier` cells
  to generate a new value through the normal pending-edit workflow.

### Bug fixes

- **SQL Server grid editing** — fixed transaction handling for grid commits,
  added TRUE/FALSE selection for `bit` columns, and reliably close inline
  editors after refresh without misclassifying Postgres bit strings.
- **Date picker placement** — date and datetime editors now flip above cells or
  clamp horizontally when needed instead of being clipped by grid controls.
- **Resizable Messages columns** — message metadata no longer overlaps, and
  every column can be resized from its header.
- **Website delivery and sharing** — fixed Tailwind availability in production
  site builds and added a purpose-built social preview card with complete Open
  Graph and X metadata.

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
