# ADR 0004: GPUI production desktop client

- **Status:** Accepted
- **Date:** 2026-08-14

## Context

Cellar's data grid is its primary product surface. Optimising the existing
Tauri/React renderer improved ordinary tables, but wide and high-volume tables
still missed Cellar's interaction target. A throwaway GPUI spike established
that native rendering alone was not enough: row-only virtualization remained
slow at 500 columns, while row-and-column virtualization made the ordinary,
wide, and 10,000-by-500 stress cases feel smooth.

Cellar's database behaviour already lives in Rust crates. Rewriting drivers or
moving credentials through a second boundary would add risk without improving
the renderer.

## Decision

GPUI is the production desktop client. It consumes a shared `cellar-runtime`
crate containing the connection registry and application services previously
owned by the Tauri entrypoint. Drivers, typed core contracts, keychain access,
SQL generation, and database safety rules remain shared Rust code.

The grid virtualizes both axes and retains only the viewport plus small
overscan. Query results cross into UI state as bounded pages. Editors and
pending changes are keyed independently from visible cells so scrolling cannot
discard edits.

The Tauri/React client remains buildable during migration and is removed only
after every item in `docs/gpui-feature-parity.md` is verified. GPUI is pinned to
an exact version while its API is pre-1.0.

## Consequences

- Cellar has one trusted Rust runtime shared by both clients during migration.
- No database credential needs to enter UI state, logs, or plain-text config.
- Native controls, focus, accessibility, packaging, and updater behaviour must
  be implemented and tested on macOS, Linux, and Windows.
- GPUI upgrades are deliberate compatibility changes rather than automatic
  semver-range updates.
- The validated spike is performance evidence only; production code is written
  in `apps/desktop-gpui`.

