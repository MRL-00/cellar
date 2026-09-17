# Release packaging

Cellar is built as a native GPUI application and published through GitHub Releases.

## Local builds

From the repository root:

```bash
cargo build --release -p cellar-desktop-gpui
```

The optimized binary is written under `target/release/`. The release workflow performs the platform bundle, signing, notarization, updater archive, and DMG steps.

## GitHub release builds

Push a plain semver tag, without a `v` prefix:

```bash
git tag 1.0.5
git push origin 1.0.5
```

Before tagging, ensure `package.json`, the workspace version in `Cargo.toml`, and `Cargo.lock` match the tag.

The `Release` workflow currently publishes macOS Apple Silicon artifacts. It builds `cellar-desktop-gpui`, creates `Cellar.app`, signs it with Developer ID, notarizes and staples it, then uploads:

- `Cellar-mac-arm64.dmg`
- `Cellar_aarch64.app.tar.gz`
- `Cellar_aarch64.app.tar.gz.sig`
- `latest.json`

Cellar 0.3.5 is the final Intel release. Windows and Linux release artifacts remain disabled until their signing and packaging paths are tested.

## Native updater

The GPUI client fetches `latest.json`, validates version and size limits, downloads the update archive, and verifies its minisign signature before installation. The matching public key is compiled into the native updater.

The release workflow still invokes the Tauri CLI's standalone signer. This is not a Tauri runtime dependency: it preserves the existing minisign-compatible private-key and signature format used by installed Cellar releases. Replacing it requires a deliberate updater-key migration, not an application migration.

Required GitHub Actions secrets:

- `DEVELOPER_ID_CERTIFICATE_BASE64`
- `DEVELOPER_ID_CERTIFICATE_PASSWORD`
- `DEVELOPER_ID_APPLICATION_IDENTITY`
- `APP_STORE_CONNECT_API_KEY_BASE64`
- `APP_STORE_CONNECT_KEY_ID`
- `APP_STORE_CONNECT_ISSUER_ID`
- `TAURI_SIGNING_PRIVATE_KEY` — legacy secret name; contains the current minisign-compatible updater private key

Do not rotate or rename the updater key casually. Existing installations only trust the public key compiled into their build; losing the matching private key breaks their update path.

Stable releases are served through:

```text
https://github.com/MRL-00/cellar/releases/latest/download/latest.json
```

Prereleases are excluded by GitHub's `/releases/latest` redirect.
