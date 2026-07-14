# Release packaging

Cellar is packaged with Tauri and published through GitHub Releases.

## Local builds

From the repository root:

```bash
pnpm install
pnpm --filter @cellar/desktop build:tauri
```

Tauri writes installers and bundles under:

```text
apps/desktop/src-tauri/target/release/bundle/
```

The exact formats depend on the host platform. For example, macOS builds produce app/dmg artifacts.

## GitHub release builds

Push a version tag to build release artifacts for macOS:

```bash
git tag 0.1.0
git push origin 0.1.0
```

Tags must be plain semver-style versions such as `0.1.0` or
`0.1.0-alpha.1`. Do not prefix release tags with `v`.
Before tagging, make sure the package metadata in `package.json`,
`apps/desktop/package.json`, `apps/desktop/src-tauri/tauri.conf.json`,
`Cargo.toml`, and `Cargo.lock` matches the tag.

The `Release` workflow builds macOS Apple Silicon artifacts only. Cellar 0.3.5
is the final Intel release; its installers remain available from that GitHub
Release, but Intel Macs do not receive later updates. The workflow signs and
notarizes the app with Developer ID, creates a draft GitHub Release while the
installer uploads, then publishes the release after the build finishes
successfully. Tags with prerelease
suffixes, such as `0.1.0-alpha.1`, are marked as prereleases.

To publish an already-created draft manually:

```bash
gh release edit 0.1.0 --draft=false
```

## In-app auto-updates

Cellar ships with the Tauri updater plugin enabled. Each release build signs
its update bundle with an Ed25519 private key; the matching public key is
embedded in `tauri.conf.json` so the app can verify updates before installing.

The frontend "Check now" button in Settings → Updates calls the updater, and
"Download & install" applies the signed bundle and relaunches the app.

### Required GitHub Actions secrets

Release signing and notarization require these GitHub Actions secrets:

- `DEVELOPER_ID_CERTIFICATE_BASE64` - base64-encoded `.p12` export of the
  Developer ID Application certificate and private key.
- `DEVELOPER_ID_CERTIFICATE_PASSWORD` - password for the `.p12` export.
- `DEVELOPER_ID_APPLICATION_IDENTITY` - codesigning identity, for example
  `Developer ID Application: MRL TECHNOLOGY LIMITED (U5LM2CZXRN)`.
- `APP_STORE_CONNECT_API_KEY_BASE64` - base64-encoded App Store Connect API
  key `.p8` file.
- `APP_STORE_CONNECT_KEY_ID` - App Store Connect API key ID.
- `APP_STORE_CONNECT_ISSUER_ID` - App Store Connect issuer ID.
- `TAURI_SIGNING_PRIVATE_KEY` - the contents of the Tauri updater private key
  (generated with `tauri signer generate`). Keep a backup off-repo.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` - the password for the private key, or
  empty if the key was generated without one.

### How the manifest is published

`tauri-action` generates and uploads a `latest.json` manifest to each GitHub
Release. The app fetches it from
`https://github.com/MRL-00/cellar/releases/latest/download/latest.json`.
Prereleases are excluded from the `/releases/latest` redirect, so only stable
releases are served to the updater. The manifest maps `darwin-aarch64` to its
signed Apple Silicon bundle and intentionally omits `darwin-x86_64` after
0.3.5.

### Rotating or losing the signing key

The public key is baked into every shipped build. If the private key is lost,
already-installed builds cannot verify updates from a new keypair. Keep the
private key backed up in a secure location (for example, a password manager
or secrets vault). A new keypair only takes effect for builds shipped after
the public key is swapped in `tauri.conf.json`.

## Website downloads

The website presents a direct Apple Silicon download. It falls back to the
GitHub Releases page, then upgrades the button to the latest direct DMG link
when the public GitHub Releases API returns a matching asset. Intel users can
continue using the preserved 0.3.5 release.

The release workflow uploads a stable alias for the Tauri-generated DMG:

- `Cellar-mac-arm64.dmg` for Apple Silicon.

The website also recognizes versioned names such as
`Cellar_0.1.0_aarch64.dmg`. This keeps downloads working for prereleases and
early releases where GitHub's `/releases/latest` endpoint may not resolve.

## Before public distribution

Only macOS builds are published today; Windows and Linux should be added after they are tested:

- macOS: Apple Developer ID signing and notarization.
- Windows: code-signing certificate.
- Linux: package metadata review for AppImage/deb/rpm outputs.

Do not enable Tauri auto-update on Windows/Linux until those platforms have
tested signed bundles and matching manifest entries in `latest.json`.
