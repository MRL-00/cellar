# Cellar for Codex

Browse, query and edit databases in Codex. Cellar runs a local MCP server and an interactive database workspace; it uses Codex for chat instead of including a second AI assistant.

## Install the public preview

Supports **Apple Silicon macOS 13+**, with a Codex installation that supports plugins and MCP Apps. Intel Mac, Windows, Linux and ChatGPT web are not supported by this package. Tested with Codex CLI 0.156.0 and its Node 24 runtime; Node 20+ is required if Codex does not supply one.

```sh
codex plugin marketplace add MRL-00/cellar --ref cellar-plugin-v0.5.2
codex plugin add cellar-browser@cellar
```

Restart Codex or start a fresh plugin-enabled conversation, then ask **Open the Cellar database workspace**. The display name is **Cellar**; `cellar-browser` remains the stable internal identifier.

The [0.5.2 release](https://github.com/MRL-00/cellar/releases/tag/cellar-plugin-v0.5.2) contains a self-contained Rust service, bundled MCP server, UI and synthetic SQLite database. First launch downloads the runtime from this repository's GitHub release, verifies its pinned SHA-256 and installs it under `~/.cellar/extensions/cellar-browser/runtimes/`. Subsequent launches use the verified local cache. The first launch needs HTTPS access to GitHub and its release download domains; database requests run locally. Users do not need Rust, npm or the native Cellar app. This is a development preview without desktop signing/notarization.

If first launch fails, inspect the Cellar MCP server logs for a download, checksum or unsupported-platform error. Retry after resolving connectivity. A corrupt cache is rejected; remove only the affected version directory printed in the logs and retry. Never disable Gatekeeper or change Codex security permissions to install it.

This GitHub distribution is publicly installable, but is not a listing in OpenAI's curated plugin directory. See [OpenAI's plugin documentation](https://learn.chatgpt.com/docs/plugins) for directory publishing requirements and availability.

## Connections and files

The bundled demo is synthetic and read-only. **Add connection** supports local SQLite paths, SQLite file imports, PostgreSQL, Supabase and Neon. Remote connections require verified TLS; Supabase uses a PostgreSQL direct or session-pooler endpoint. Passwords are sent through the local form and stored through `cellar-secrets` in the OS keychain under extension-specific identities. Native Cellar connections and credentials are not imported. Blank passwords in **Edit connection** preserve existing secrets.

**Find chat databases** asks Codex to materialize a user-selected SQLite attachment and open it with `cellar_attach`. The plugin cannot enumerate conversations itself; file access depends on the host's available tools. Imported files become private working copies and the original stays unchanged. **Download database** exports the committed copy. Import/export cap at 64 MiB; larger local databases can be connected by path.

**Connect from this project** first selects the current project's explicit root. The workspace separately asks for bounded discovery consent, displays safe candidate metadata, and requires explicit confirmation before opening a read-only connection. Secrets remain in local process memory; unresolved variables, symlinks and unsupported formats are skipped.

Profiles and working copies live under `~/.cellar/extensions/cellar-browser`. `CELLAR_EXTENSION_STATE` overrides that location; `CELLAR_EXTENSION_CONFIG` optionally supplies a metadata-only alias configuration. Do not put passwords in JSON or chat.

## Workspace

- Searchable schema tree; table and SQL tabs; server-side filters, sorting and bounded paging; indexes, foreign keys and column details.
- SQL completion/highlighting, cursor or selection execution, SELECT/WITH batches, history and EXPLAIN without ANALYZE.
- CSV, TSV and JSON export of the loaded page, SQLite downloads, cell inspection, cancellation and dark/light themes.
- Stage row inserts, edits and deletions, inspect generated SQL in **Review & Commit**, and type the exact confirmation. Writes require an explicitly editable connection and run in one transaction with primary-key and original-row checks.

Rows and connection details are delivered to the UI through tool metadata; model-facing results contain counts/status. The Codex host transports the data. Requests cap at 500 rows, 128 columns, 16 KiB cells, 512 KiB payload and 1 MiB transport. Use trusted schemas and least-privilege roles. SQL validation and read-only transactions constrain ordinary queries; arbitrary write/DDL SQL is unavailable. Timeouts can leave a commit outcome uncertain: inspect rows before retrying. Commits never retry automatically.

## Known limits

The UI follows native Cellar's assets, themes, dialogs and menus, with unsupported actions visibly disabled. It is not a complete native port. SSH tunnels, migrations/schema diff, ER diagrams, rich binary editors, native connection import, persistent query history/layout and split panes are unavailable. Pending edits do not survive host closure. PostgreSQL table DDL reconstruction is incomplete. SQLite binary values are not editable; JavaScript numbers have JavaScript precision; separate pages are not a repeatable snapshot.

Browser tests verify the workspace. Codex protocol tests verify registration, tools and the UI resource. Visible native host rendering is a separate check and has not been automated. No real database credentials are used in tests.

## Develop and package

From the repository root:

```sh
cargo build --locked -p cellar-extension-service
cd apps/cellar-browser
npm ci
npm run build
npm run fixtures
npm run preview
```

Open the loopback URL printed by the preview. It serves the same bundled HTML as MCP, but is a development preview rather than a plugin installation. Fixtures refuse to overwrite existing databases.

For the Apple Silicon release, build the release service, then run:

```sh
cargo build --locked --release -p cellar-extension-service # repository root
cd apps/cellar-browser
CELLAR_SERVICE_BINARY="$PWD/../../target/release/cellar-extension-service" npm run build
npm run fixtures # once, if no fixture exists
node scripts/package-release.mjs
```

The packager writes ignored archives and checksums in `release-artifacts/`, and updates the tracked marketplace runtime manifest, plugin manifest, skill and icon. Commit those source files, tag `cellar-plugin-vVERSION`, and attach the archive and `SHA256SUMS` to the matching GitHub release. Plugin tags deliberately do not match the desktop app release workflow. Never commit generated runtime binaries or private state.

See [validation commands and coverage](VALIDATION.md).
