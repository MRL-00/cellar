use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use base64::Engine as _;
use futures_util::StreamExt as _;
use gpui::{div, prelude::*, px, AnyElement, Context, Timer};
use gpui_component::Icon;
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::Deserialize;

use super::CellarApp;
use crate::app::settings::SettingsCategory;
use cellar_desktop_gpui::theme::{ACCENT, ACCENT_FG, BORDER, FG, FG_MUTED, FG_SECONDARY, PANEL};

const ENDPOINT: &str = "https://github.com/MRL-00/cellar/releases/latest/download/latest.json";
const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
const MAX_UPDATE_BYTES: usize = 1024 * 1024 * 1024;
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEU1QjRGMjE3RTI1ODYwOEQKUldTTllGamlGL0swNWRCNnE5enNpTUZycE5wZU9CWDlpbm9EcEFxQzFYN3BBZ3pCQ0hvMWV5SHQK";

#[derive(Clone, Debug)]
pub(super) struct UpdateInfo {
    pub(super) version: String,
    pub(super) notes: Option<String>,
    url: String,
    signature: String,
}

#[derive(Clone, Debug, Default)]
pub(super) enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    Available(UpdateInfo),
    UpToDate,
    Downloading(f32),
    Installing,
    Error(String),
}

impl UpdateStatus {
    pub(super) fn label(&self) -> String {
        match self {
            Self::Idle => "Ready".into(),
            Self::Checking => "Checking…".into(),
            Self::Available(update) => format!("Update available: v{}", update.version),
            Self::UpToDate => "Up to date".into(),
            Self::Downloading(fraction) => format!("Downloading… {:.0}%", fraction * 100.),
            Self::Installing => "Installing…".into(),
            Self::Error(error) => format!("Error: {error}"),
        }
    }

    pub(super) fn can_check(&self) -> bool {
        !matches!(
            self,
            Self::Checking | Self::Available(_) | Self::Downloading(_) | Self::Installing
        )
    }

    pub(super) fn can_install(&self) -> bool {
        matches!(self, Self::Available(_)) && packaged_app_path().is_ok()
    }
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    notes: Option<String>,
    platforms: HashMap<String, Platform>,
}

#[derive(Deserialize)]
struct Platform {
    url: String,
    signature: String,
}

impl CellarApp {
    pub(super) fn show_update_toast(&self) -> bool {
        matches!(&self.updater_status, UpdateStatus::Available(update)
            if self.dismissed_update_version.as_deref() != Some(update.version.as_str()))
            && !self.settings_open
    }

    pub(super) fn update_toast(&self, cx: &mut Context<Self>) -> AnyElement {
        let UpdateStatus::Available(update) = &self.updater_status else {
            return div().into_any_element();
        };
        let version = update.version.clone();
        let dismiss_version = version.clone();
        div()
            .absolute()
            .bottom(px(16.))
            .right(px(16.))
            .w(px(280.))
            .rounded(px(7.))
            .border_1()
            .border_color(BORDER)
            .bg(PANEL)
            .p_3()
            .shadow_lg()
            .child(
                div()
                    .mb_2()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        Icon::empty()
                            .path("icons/sparkles.svg")
                            .size(px(14.))
                            .text_color(ACCENT),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(FG)
                                    .child("Update available"),
                            )
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .text_color(FG_SECONDARY)
                                    .child(format!("Version {version} is ready to download.")),
                            ),
                    )
                    .child(
                        div()
                            .id("dismiss-update-toast")
                            .tab_index(0)
                            .cursor_pointer()
                            .size(px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(FG_MUTED)
                            .child(Icon::empty().path("icons/close.svg").size(px(12.)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.dismissed_update_version = Some(dismiss_version.clone());
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("open-update-settings")
                    .tab_index(0)
                    .cursor_pointer()
                    .h(px(26.))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .rounded(px(4.))
                    .bg(ACCENT)
                    .text_size(px(14.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(ACCENT_FG)
                    .hover(|style| {
                        style.bg(cellar_desktop_gpui::theme::hover_bright(ACCENT.rgba()))
                    })
                    .child(Icon::empty().path("icons/download.svg").size(px(11.)))
                    .child("Update")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.dismissed_update_version = Some(version.clone());
                        this.open_settings(SettingsCategory::Updates, cx);
                    })),
            )
            .into_any_element()
    }

    pub(crate) fn initialize_updater(&mut self, cx: &mut Context<Self>) {
        self.updater_task = Some(cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_secs(2)).await;
            this.update(cx, |this, cx| this.check_for_updates(cx)).ok();
        }));
    }

    pub(super) fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if !self.updater_status.can_check() {
            return;
        }
        self.updater_status = UpdateStatus::Checking;
        let runtime = Arc::clone(&self.runtime);
        cx.notify();
        self.updater_task = Some(cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(check())
                .await
                .map_err(|error| format!("update check task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.updater_status = match result {
                    Ok(Some(update)) => UpdateStatus::Available(update),
                    Ok(None) => UpdateStatus::UpToDate,
                    Err(error) => UpdateStatus::Error(error),
                };
                this.updater_last_checked = Some(chrono::Local::now().format("%x, %X").to_string());
                cx.notify();
            })
            .ok();
        }));
    }

    pub(super) fn download_and_install_update(&mut self, cx: &mut Context<Self>) {
        let UpdateStatus::Available(update) = self.updater_status.clone() else {
            return;
        };
        self.updater_status = UpdateStatus::Downloading(0.);
        let runtime = Arc::clone(&self.runtime);
        let (sender, receiver) = async_channel::unbounded();
        runtime.spawn(async move {
            let result = download_and_install(update, sender.clone()).await;
            sender.send(UpdateEvent::Finished(result)).await.ok();
        });
        self.updater_task = Some(cx.spawn(async move |this, cx| {
            while let Ok(event) = receiver.recv().await {
                let finished = matches!(event, UpdateEvent::Finished(_));
                this.update(cx, |this, cx| match event {
                    UpdateEvent::Downloading(fraction) => {
                        this.updater_status = UpdateStatus::Downloading(fraction);
                        cx.notify();
                    }
                    UpdateEvent::Installing => {
                        this.updater_status = UpdateStatus::Installing;
                        cx.notify();
                    }
                    UpdateEvent::Finished(Ok(executable)) => {
                        match std::process::Command::new(executable).spawn() {
                            Ok(_) => cx.quit(),
                            Err(error) => {
                                this.updater_status = UpdateStatus::Error(format!(
                                    "update installed but relaunch failed: {error}"
                                ));
                                cx.notify();
                            }
                        }
                    }
                    UpdateEvent::Finished(Err(error)) => {
                        this.updater_status = UpdateStatus::Error(error);
                        cx.notify();
                    }
                })
                .ok();
                if finished {
                    break;
                }
            }
        }));
    }
}

enum UpdateEvent {
    Downloading(f32),
    Installing,
    Finished(Result<PathBuf, String>),
}

async fn check() -> Result<Option<UpdateInfo>, String> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("Cellar/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| error.to_string())?
        .get(ENDPOINT)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_MANIFEST_BYTES as u64)
    {
        return Err("update manifest is unexpectedly large".into());
    }
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err("update manifest is unexpectedly large".into());
    }
    select_update(&bytes, env!("CARGO_PKG_VERSION"), platform_target())
}

fn select_update(bytes: &[u8], current: &str, target: &str) -> Result<Option<UpdateInfo>, String> {
    let mut manifest: Manifest =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let remote = Version::parse(manifest.version.trim_start_matches('v'))
        .map_err(|error| format!("invalid update version: {error}"))?;
    let current = Version::parse(current.trim_start_matches('v'))
        .map_err(|error| format!("invalid app version: {error}"))?;
    if remote <= current {
        return Ok(None);
    }
    let platform = manifest
        .platforms
        .remove(target)
        .ok_or_else(|| format!("release does not support {target}"))?;
    if !platform.url.starts_with("https://") {
        return Err("update URL must use HTTPS".into());
    }
    if platform.signature.trim().is_empty() {
        return Err("update signature is missing".into());
    }
    Ok(Some(UpdateInfo {
        version: remote.to_string(),
        notes: manifest.notes,
        url: platform.url,
        signature: platform.signature,
    }))
}

async fn download_and_install(
    update: UpdateInfo,
    progress: async_channel::Sender<UpdateEvent>,
) -> Result<PathBuf, String> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent(concat!("Cellar/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| error.to_string())?
        .get(&update.url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let total = response.content_length();
    if total.is_some_and(|length| length > MAX_UPDATE_BYTES as u64) {
        return Err("update package is unexpectedly large".into());
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| error.to_string())?;
        if bytes.len().saturating_add(chunk.len()) > MAX_UPDATE_BYTES {
            return Err("update package is unexpectedly large".into());
        }
        bytes.extend_from_slice(&chunk);
        if let Some(total) = total.filter(|total| *total > 0) {
            progress
                .send(UpdateEvent::Downloading(bytes.len() as f32 / total as f32))
                .await
                .ok();
        }
    }
    verify_signature(&bytes, &update.signature)?;
    progress.send(UpdateEvent::Installing).await.ok();
    install(&bytes)
}

fn verify_signature(bytes: &[u8], signature: &str) -> Result<(), String> {
    let public_key = base64::engine::general_purpose::STANDARD
        .decode(PUBLIC_KEY)
        .map_err(|error| error.to_string())?;
    let public_key = std::str::from_utf8(&public_key).map_err(|error| error.to_string())?;
    let public_key = PublicKey::decode(public_key).map_err(|error| error.to_string())?;
    let signature = base64::engine::general_purpose::STANDARD
        .decode(signature)
        .map_err(|error| error.to_string())?;
    let signature = std::str::from_utf8(&signature).map_err(|error| error.to_string())?;
    let signature = Signature::decode(signature).map_err(|error| error.to_string())?;
    public_key
        .verify(bytes, &signature, true)
        .map_err(|error| format!("update signature verification failed: {error}"))
}

#[cfg(target_os = "macos")]
fn install(bytes: &[u8]) -> Result<PathBuf, String> {
    use flate2::read::GzDecoder;

    let current_app = packaged_app_path()?;
    let parent = current_app
        .parent()
        .ok_or_else(|| "Cellar.app has no parent directory".to_string())?;
    let extraction = tempfile::Builder::new()
        .prefix(".cellar-update-")
        .tempdir_in(parent)
        .map_err(|error| error.to_string())?;
    tar::Archive::new(GzDecoder::new(bytes))
        .unpack(extraction.path())
        .map_err(|error| error.to_string())?;
    let new_app = extraction.path().join("Cellar.app");
    let new_executable = new_app.join("Contents/MacOS/Cellar");
    if !new_executable.is_file() {
        return Err("update archive does not contain Cellar.app".into());
    }
    let verified = std::process::Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&new_app)
        .status()
        .map_err(|error| error.to_string())?;
    if !verified.success() {
        return Err("downloaded Cellar.app failed code-signature verification".into());
    }
    let backup_root = tempfile::Builder::new()
        .prefix(".cellar-backup-")
        .tempdir_in(parent)
        .map_err(|error| error.to_string())?;
    let backup = backup_root.path().join("Cellar.app");
    std::fs::rename(&current_app, &backup).map_err(|error| error.to_string())?;
    if let Err(error) = std::fs::rename(&new_app, &current_app) {
        std::fs::rename(&backup, &current_app).map_err(|rollback| {
            format!("install failed: {error}; rollback also failed: {rollback}")
        })?;
        return Err(format!("install failed: {error}"));
    }
    Ok(current_app.join("Contents/MacOS/Cellar"))
}

#[cfg(not(target_os = "macos"))]
fn install(_: &[u8]) -> Result<PathBuf, String> {
    Err("the signed updater is currently available for macOS only".into())
}

fn packaged_app_path() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|error| error.to_string())?
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(PathBuf::from)
        .ok_or_else(|| "updates can only be installed from a packaged Cellar.app".into())
}

fn platform_target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-aarch64",
        ("macos", "x86_64") => "darwin-x86_64",
        ("windows", "x86_64") => "windows-x86_64",
        ("linux", "x86_64") => "linux-x86_64",
        _ => "unsupported",
    }
}

#[cfg(test)]
mod tests {
    use super::select_update;

    #[test]
    fn selects_only_newer_https_platform_updates() {
        let json = br#"{
          "version":"2.0.0",
          "notes":"new",
          "platforms":{"darwin-aarch64":{"url":"https://example.com/app.tar.gz","signature":"sig"}}
        }"#;
        let update = select_update(json, "1.0.0", "darwin-aarch64")
            .unwrap()
            .unwrap();
        assert_eq!(update.version, "2.0.0");
        assert!(select_update(json, "2.0.0", "darwin-aarch64")
            .unwrap()
            .is_none());
        assert!(select_update(json, "1.0.0", "linux-x86_64").is_err());
    }
}
