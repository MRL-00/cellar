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

The exact formats depend on the host platform. For example, macOS builds produce app/dmg artifacts, Windows builds produce installer artifacts, and Linux builds produce Linux package formats.

## GitHub release builds

Push a version tag to build release artifacts for macOS, Windows, and Linux:

```bash
git tag cellar-v0.1.0-alpha
git push origin cellar-v0.1.0-alpha
```

The `Release` workflow creates a draft prerelease and uploads the Tauri installers. Review the draft, edit release notes, then publish it.

The website download button should link to the latest GitHub Release or to a small platform-detection page that redirects to the right release asset for macOS, Windows, or Linux.

## Before public distribution

Unsigned builds are useful for early testers, but public downloads need platform signing:

- macOS: Apple Developer ID signing and notarization.
- Windows: code-signing certificate.
- Linux: package metadata review for AppImage/deb/rpm outputs.

Do not enable Tauri auto-update until release signing keys and updater metadata are deliberately configured.
