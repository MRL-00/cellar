# Changelog

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
