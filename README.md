# Cellar

An open-source, cross-platform desktop database client with AI in the workflow, not stapled to the side.

![Cellar main window](apps/site/public/assets/cellar-main.png)

Cellar is a dense, keyboard-first SQL workspace for developers, DBAs, and analysts. It is desktop-first, MIT licensed, local-first, and private by default.

## Desktop stack

Cellar 1.0 uses a native Rust desktop stack:

- GPUI 0.2.2 for the shell, editor, controls, and two-axis virtualized grid.
- `cellar-runtime` for application, connection, query, history, and transaction services.
- `cellar-core` and first-party driver crates for typed database behavior.
- `cellar-secrets` for OS-keychain credential storage.
- A native signed updater with bounded downloads and minisign verification.

The former Tauri/React parity client was removed after the GPUI client shipped. A read-only migration path remains so older installations can carry their WebKit-stored layout and appearance preferences forward.

The React/Vite code under `apps/site` is the marketing and download site, not the desktop client.

## Quick start

Prerequisites:

- Rust stable
- Node.js 20 or newer and pnpm 9 or newer for the website and repository checks
- macOS users: Xcode command-line tools

```bash
git clone https://github.com/MRL-00/cellar.git
cd cellar
pnpm install
pnpm dev
```

Useful commands:

```bash
pnpm dev                 # native GPUI desktop client
pnpm build:native        # optimized GPUI binary
cargo check --workspace
cargo test --workspace
pnpm build               # website and remaining JS workspaces
pnpm typecheck
pnpm lint
pnpm test
```

Run the website locally with `pnpm --filter @cellar/site dev`.

## Repository layout

```text
cellar/
├── apps/
│   ├── desktop-gpui/    # Production native desktop client and its assets
│   └── site/            # Marketing and download site
├── crates/
│   ├── cellar-runtime/  # Shared application and connection services
│   ├── cellar-core/     # Driver traits, errors, schema/query types
│   ├── cellar-drivers/  # First-party database drivers
│   ├── cellar-sql/      # SQL parsing and dialect support
│   ├── cellar-diff/     # Pending row edits to transactional SQL
│   ├── cellar-schema-diff/
│   ├── cellar-ai/       # Native provider transports and auth
│   └── cellar-secrets/  # Keychain and credential storage
├── docs/                # Architecture and release documentation
└── SPEC.md              # Canonical product and architecture spec
```

The runtime is direct and typed:

```text
GPUI entities and models
  -> cellar-runtime services
  -> cellar-core driver traits
  -> concrete database drivers
```

Credentials remain in `cellar-secrets`; connection configuration must never contain passwords or provider keys.

## Contributing

Read [SPEC.md](SPEC.md) before architectural or product changes. Prefer small vertical slices, existing patterns, and source files below the repository's 800-line limit.

Before opening a pull request:

```bash
pnpm lint
pnpm test
cargo check --workspace
```

Use Conventional Commit-style PR titles such as `feat: add scenario templates` or `fix: handle titlebar double-click`.

## License

[MIT](LICENSE)
