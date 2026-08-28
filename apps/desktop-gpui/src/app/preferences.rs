use serde::{Deserialize, Serialize};

use super::sidebar_layout::{
    prefers_dev_layout, read_local_storage_value, tauri_local_storage_paths,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum Theme {
    System,
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum Density {
    #[default]
    Compact,
    Comfortable,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct EditorPreferences {
    pub tab_size: usize,
    pub soft_wrap: bool,
    pub line_numbers: bool,
    pub bracket_matching: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct GridPreferences {
    pub null_display: String,
    pub stripe_rows: bool,
    pub remember_table_sort: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Preferences {
    pub(super) theme: Theme,
    pub(super) density: Density,
    pub(super) accent: String,
    pub(super) font_size_px: f32,
    pub(super) interface_font: String,
    pub(super) mono_font: String,
    pub(super) editor: EditorPreferences,
    pub(super) grid: GridPreferences,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            density: Density::Compact,
            accent: "#a78bfa".into(),
            font_size_px: 13.5,
            interface_font: "SF Pro Text".into(),
            mono_font: "JetBrains Mono".into(),
            editor: EditorPreferences {
                tab_size: 4,
                soft_wrap: false,
                line_numbers: true,
                bracket_matching: true,
            },
            grid: GridPreferences {
                null_display: "NULL".into(),
                stripe_rows: false,
                remember_table_sort: true,
            },
        }
    }
}

impl Preferences {
    pub(crate) async fn load_classic() -> Self {
        let mut candidates = Vec::new();
        for path in tauri_local_storage_paths() {
            let Some(json) = read_local_storage_value(&path, "cellar.settings.v1").await else {
                continue;
            };
            let Some(settings) = serde_json::from_str::<Self>(&json).ok() else {
                continue;
            };
            let modified = path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            candidates.push((prefers_dev_layout(&path), modified, settings.sanitized()));
        }
        candidates
            .into_iter()
            .max_by_key(|(preferred, modified, _)| (*preferred, *modified))
            .map(|(_, _, settings)| settings)
            .unwrap_or_default()
    }

    pub(super) fn sanitized(mut self) -> Self {
        if !self.font_size_px.is_finite() {
            self.font_size_px = 13.5;
        }
        self.font_size_px = self.font_size_px.clamp(10., 22.);
        if ![2, 4, 8].contains(&self.editor.tab_size) {
            self.editor.tab_size = 4;
        }
        if !["NULL", "∅", "(empty)"].contains(&self.grid.null_display.as_str()) {
            self.grid.null_display = "NULL".into();
        }
        self
    }
}

pub(super) fn parse_font_size(raw: &str) -> Option<f32> {
    raw.parse::<f32>()
        .ok()
        .filter(|value| (10. ..=22.).contains(value))
}

#[cfg(test)]
mod tests {
    use super::{parse_font_size, Preferences};

    #[test]
    fn malformed_preferences_fall_back_to_classic_bounds() {
        let settings = Preferences {
            font_size_px: f32::NAN,
            editor: super::EditorPreferences {
                tab_size: 3,
                ..Preferences::default().editor
            },
            grid: super::GridPreferences {
                null_display: "wat".into(),
                ..Preferences::default().grid
            },
            ..Preferences::default()
        }
        .sanitized();
        assert_eq!(settings.font_size_px, 13.5);
        assert_eq!(settings.editor.tab_size, 4);
        assert_eq!(settings.grid.null_display, "NULL");
    }

    #[test]
    fn editable_font_size_accepts_only_the_classic_range() {
        assert_eq!(parse_font_size("13.5"), Some(13.5));
        assert_eq!(parse_font_size("9.5"), None);
        assert_eq!(parse_font_size("large"), None);
    }
}
