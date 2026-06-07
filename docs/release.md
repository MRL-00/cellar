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
git tag 0.1.0
git push origin 0.1.0
```

Tags must be plain semver-style versions such as `0.1.0` or
`0.1.0-alpha.1`. Do not prefix release tags with `v`.
Before tagging, make sure the package metadata in `package.json`,
`apps/desktop/package.json`, `apps/desktop/src-tauri/tauri.conf.json`,
`Cargo.toml`, and `Cargo.lock` matches the tag.

The `Release` workflow creates a draft GitHub Release and uploads the Tauri installers. Tags with prerelease suffixes, such as `0.1.0-alpha.1`, are marked as prereleases. Review the draft, edit release notes, then publish it.

## Website links

The simplest website download button should link to:

```text
https://github.com/MRL-00/cellar/releases/latest
```

GitHub redirects that URL to the latest published release. This is the most
robust link while installer filenames include the version number.

For a direct platform download button, either use a small platform-detection
page that reads the latest GitHub Release assets, or upload stable asset names
such as `Cellar-mac-arm64.dmg` in addition to Tauri's versioned artifacts.
Stable asset names can then be linked with:

```text
https://github.com/MRL-00/cellar/releases/latest/download/Cellar-mac-arm64.dmg
```

## Before public distribution

Unsigned builds are useful for early testers, but public downloads need platform signing:

- macOS: Apple Developer ID signing and notarization.
- Windows: code-signing certificate.
- Linux: package metadata review for AppImage/deb/rpm outputs.

Do not enable Tauri auto-update until release signing keys and updater metadata are deliberately configured.
