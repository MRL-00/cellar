# Connect from this project (0.5.0)

Select **Connect from this project** in the sidebar. In Codex, **Use current
Codex project** asks the host agent to call `cellar_project` with only the active
task folder from its environment context. Alternatively enter a folder explicitly.
The workspace shows the selected absolute folder and asks permission to discover
local settings. Discovery does not connect. Choose one candidate and confirm
**Connect read-only** before the workspace requests its schema or rows.

## Context evidence

A synthetic ephemeral Codex app-server probe on CLI 0.156.0 advertised
`experimental.codex/auth-change` and `elicitation`, without MCP `roots`.
Tool metadata contained `progressToken` and `threadId`, without a project path.
The probe's MCP working directory remained the explicitly configured plugin
directory, while the task working directory was a different synthetic folder.
Therefore this release does not infer the active project from server cwd, parent
directories, process environment, session history or filesystem searches. The
supported handoff is a normal MCP tool call with an explicit root. If the active
task root is unavailable or ambiguous, the host must ask the user to choose it.
The workspace confirms the root before reading project configuration.

## Supported formats

- Root-level `.env`, `.env.local`, `.env.development`, `.env.test`,
  `.env.staging` and `.env.production`; literal `DATABASE_URL`, `POSTGRES_URL`,
  `POSTGRESQL_URL` and `SQLITE_URL` PostgreSQL URLs or `file:` SQLite paths.
- Literal `SQLITE_PATH` and `DATABASE_PATH` paths; `DB_CONNECTION=sqlite` with
  `DB_DATABASE`; PostgreSQL `PGHOST/PGPORT/PGDATABASE/PGUSER/PGPASSWORD/PGSSLMODE`,
  or `DB_CONNECTION=pgsql|postgres|postgresql` with
  `DB_HOST/DB_PORT/DB_DATABASE/DB_USERNAME/DB_PASSWORD/DB_SSLMODE`.
- Strict root-level `appsettings.json`, `appsettings.Development.json`,
  `appsettings.Staging.json`, `appsettings.Production.json`; `ConnectionStrings`
  values containing supported PostgreSQL URLs, simple Npgsql-style key/value
  strings, or `Data Source=...` SQLite strings.
- Existing `.db`, `.sqlite` and `.sqlite3` files with a valid SQLite header,
  in the root or up to two nested directory levels.

No YAML/Compose, JS/Python config execution, shell sourcing, JSON comments,
escaped/quoted semicolon connection-string grammar, external certificate files,
process environment substitution, dotenv interpolation or secret-manager lookup.
Unsupported syntax and unresolved variables are skipped, with generic warnings.
PostgreSQL container service names are not translated into published localhost
ports. Root-level config only is intentional; monorepos can explicitly select
the application subfolder. Unspecified host/database/user values are not guessed.

## Boundaries and lifetime

The scan allows at most 400 directory entries, 32 candidates and 64 KiB per
configuration file. It skips symlinks, hidden folders and dependency/build folders.
SQLite paths must stay in the selected root and contain no traversal segments.
Path components and SQLite headers are revalidated before every database request.
Changed source configuration invalidates the selection. Detection never executes
project scripts or SQL. Read-only sessions cannot commit, save profiles or export
the project database through the editing/download path.

The UI receives source filename, engine, target category, availability and a
production-like flag. Connection URLs, hosts, database users, passwords and raw
configuration remain in local server memory, never tool content, UI props or logs.
The server supplies the ephemeral profile and credential to its local Rust child
over stdin; it never saves them to a file or the OS keychain. Sessions expire after
15 minutes or when the server exits, the project changes, or the session disconnects.
Expired state is removed by the next request and a periodic local cleanup.

Both SQLite and PostgreSQL run read-only at driver/transaction level; write routes
are rejected in Node and Rust. Remote PostgreSQL always verifies TLS certificates
and hostnames. Production-like names require additional acknowledgement, while
all other targets remain **Environment unverified**: localhost or a development
filename never establishes safety.

## Distribution and verification

The public 0.5.2 preview includes this capability. Every discovery and connection
still requires the workspace's explicit local consent and confirmation flow.
Installing a plugin does not authorize reading arbitrary project configuration.
Native Codex rendering and the host-agent project handoff are separate visual
checks; app-server tool calls and synthetic browser preview checks provide
protocol and workflow evidence without proving visible native host rendering.
