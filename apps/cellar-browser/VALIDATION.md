# Cellar plugin validation

Use only synthetic databases. Do not import native Cellar credentials or profiles.

## Source and service

```sh
cargo test --locked -p cellar-extension-service -p cellar-diff -p cellar-core -p cellar-sql
cargo fmt -p cellar-extension-service --check
cargo clippy --locked -p cellar-extension-service --no-deps -- -D warnings
cd apps/cellar-browser
npm ci
npm run fixtures
npm run build
npm run lint
npm test
```

Service tests cover bounded reads, SQL rejection, transactional row edits, rollback, revision conflicts, connection metadata, import/export, session ownership and project discovery safeguards. Optional PostgreSQL tests require a separately prepared disposable fixture; enable with `CELLAR_TEST_POSTGRES=1`. Tests never need a real database.

## Browser workspace

Run `python3 scripts/project-fixtures.py` once, start `npm run preview` and run `npm run test:ui`. Set `CELLAR_TEST_PREVIEW_URL` when using a different loopback port. Browser tests cover grid geometry, filters, SQL, menus, connection dialogs, imports, editing review, pending-change guards and theme behavior. Browser preview checks are distinct from visible rendering in Codex.

## Public runtime installer

```sh
node --test plugins/cellar-browser/tests/runtime.test.mjs # repository root
```

Installer tests cover unsupported platforms, cold and warm cache behavior, tampered files, checksum mismatch and unexpected archive entries. A release should also be tested by adding the tagged GitHub marketplace in an isolated Codex home and launching its installed MCP server against the bundled synthetic database.

The UI resource and MCP tools can be checked through `scripts/check-codex-host.mjs` in a fresh ephemeral Codex app-server thread. This checks the protocol, not the native app's visible panel, icon or attachment button.
