# Cellar

An open-source desktop database client for people who spend a lot of time in SQL.

![Cellar main window](apps/site/public/assets/cellar-main.png)

Cellar keeps schema browsing, queries, results, and pending data changes in one native workspace. It is keyboard-first, MIT licensed, local by default, and built in Rust with [GPUI](https://www.gpui.rs/).

It is also early-access software. The useful database workflow is here, but there are still rough edges and unfinished parts.

## Download

[Download the latest release](https://github.com/MRL-00/cellar/releases/latest). The signed build currently supports Apple Silicon Macs running macOS 13 or newer.

Windows and Linux builds are checked in CI, but release packaging for them is not ready yet. Cellar 0.3.5 was the final Intel Mac release.

## What works

- PostgreSQL, MySQL, SQLite, SQL Server, Azure SQL, Firestore, Convex, and Cosmos DB connections
- Supabase, Neon, and PlanetScale through their native database protocols
- Live schema browsing, SQL editing, completion, formatting, and query history
- A bounded, two-axis virtualized result grid for large tables
- Reviewable row edits that stay pending until you choose to commit them
- Query plans, schema comparison, ER diagrams, import/export, and a `Cmd/Ctrl+K` command palette

AI support is being built around your own provider account. Context stays visible, generated SQL stays reviewable, and Cellar does not put a hosted proxy between you and the provider.

There is no Cellar account or cloud sync. Database credentials and provider keys are stored through the operating system keychain, and telemetry is off by default.

## Build it

Prerequisites:

- Rust stable
- Node.js 20 or newer
- pnpm 9 or newer
- macOS users: Xcode command-line tools

```bash
git clone https://github.com/MRL-00/cellar.git
cd cellar
pnpm install
pnpm dev
```

`pnpm dev` runs the native GPUI desktop client. The website is a separate app and can be started with `pnpm --filter @cellar/site dev`.

Useful checks:

```bash
cargo check --workspace
cargo test --workspace
pnpm build
pnpm typecheck
pnpm lint
pnpm test
```

## How it is put together

```text
GPUI entities and models
  -> cellar-runtime services
  -> cellar-core driver traits
  -> concrete database drivers
```

The production desktop app lives in `apps/desktop-gpui`. Shared Rust services and database drivers live under `crates`, while `apps/site` contains the React/Vite marketing site. There is no browser runtime or second IPC layer in the desktop app.

Read [SPEC.md](SPEC.md) for the product and architecture decisions, and [the architecture overview](docs/architecture/overview.md) for the shorter version.

## Contributing

Issues and pull requests are welcome. Please run the relevant checks before opening a pull request:

```bash
pnpm test
cargo check --workspace
pnpm lint
```

Use a conventional prefix in the title, for example `feat: add query templates` or `fix: preserve pending row edits`.

## License

[MIT](LICENSE)
