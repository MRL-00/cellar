use std::collections::HashMap;

use cellar_core::value::ColumnMeta;
use gpui::Context;
use serde::{Deserialize, Serialize};

use super::{DataGrid, MAX_COLUMN_WIDTH, MIN_COLUMN_WIDTH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridLayout {
    columns: Vec<LayoutColumn>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PortableGridLayout {
    pub order: Vec<String>,
    pub widths: HashMap<String, f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LayoutColumn {
    name: String,
    occurrence: usize,
    width: f32,
    /// Whether `width` is a width the user chose. Layouts saved before columns
    /// fit their content have no flag, so their fixed widths are treated as
    /// defaults and the column fits its data again.
    #[serde(default)]
    exact: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ColumnKey {
    name: String,
    occurrence: usize,
}

impl DataGrid {
    pub fn layout(&self) -> GridLayout {
        GridLayout {
            columns: column_keys(&self.result.columns)
                .into_iter()
                .enumerate()
                .map(|(index, key)| LayoutColumn {
                    name: key.name,
                    occurrence: key.occurrence,
                    width: self.column_widths[index],
                    exact: self.widths_user_set.contains(&index),
                })
                .collect(),
        }
    }

    pub fn apply_layout(&mut self, layout: &GridLayout, cx: &mut Context<Self>) {
        let desired = layout
            .columns
            .iter()
            .map(|column| ColumnKey {
                name: column.name.clone(),
                occurrence: column.occurrence,
            })
            .collect::<Vec<_>>();
        let mut target = 0;
        for key in &desired {
            let current = column_keys(&self.result.columns);
            let Some(source) = current.iter().position(|candidate| candidate == key) else {
                continue;
            };
            if target < self.result.columns.len() {
                self.move_column(source, target, cx);
                target += 1;
            }
        }
        let widths = layout
            .columns
            .iter()
            .filter(|column| column.exact)
            .map(|column| {
                (
                    ColumnKey {
                        name: column.name.clone(),
                        occurrence: column.occurrence,
                    },
                    column.width.clamp(MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH),
                )
            })
            .collect::<HashMap<_, _>>();
        for (index, key) in column_keys(&self.result.columns).iter().enumerate() {
            if let Some(width) = widths.get(key) {
                std::sync::Arc::make_mut(&mut self.column_widths)[index] = *width;
                self.widths_user_set.insert(index);
            }
        }
        self.suppress_sort = false;
        cx.notify();
    }
}

impl GridLayout {
    pub fn portable(&self) -> PortableGridLayout {
        PortableGridLayout {
            order: self
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect(),
            widths: self
                .columns
                .iter()
                .map(|column| (column.name.clone(), column.width))
                .collect(),
        }
    }

    pub fn from_portable(layout: PortableGridLayout) -> Self {
        let mut occurrences = HashMap::<String, usize>::new();
        let columns = layout
            .order
            .into_iter()
            .map(|name| {
                let occurrence = occurrences.entry(name.clone()).or_default();
                // A transferred layout only pins columns it recorded a width
                // for; the rest fit their content like a fresh grid.
                let recorded = layout.widths.get(&name).copied();
                let column = LayoutColumn {
                    width: recorded
                        .unwrap_or(160.)
                        .clamp(MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH),
                    name,
                    occurrence: *occurrence,
                    exact: recorded.is_some(),
                };
                *occurrence += 1;
                column
            })
            .collect();
        Self { columns }
    }
}

fn column_keys(columns: &[ColumnMeta]) -> Vec<ColumnKey> {
    let mut occurrences = HashMap::<&str, usize>::new();
    columns
        .iter()
        .map(|column| {
            let occurrence = occurrences.entry(&column.name).or_default();
            let key = ColumnKey {
                name: column.name.clone(),
                occurrence: *occurrence,
            };
            *occurrence += 1;
            key
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use cellar_core::value::ColumnMeta;

    use super::column_keys;

    #[test]
    fn duplicate_columns_keep_stable_layout_identities() {
        let columns = ["id", "name", "id"].map(|name| ColumnMeta {
            name: name.into(),
            data_type: "text".into(),
            nullable: true,
        });
        let keys = column_keys(&columns);
        assert_eq!(keys[0].occurrence, 0);
        assert_eq!(keys[2].occurrence, 1);
    }

    #[test]
    fn portable_layouts_accept_the_classic_name_based_format() {
        let layout = super::GridLayout::from_portable(super::PortableGridLayout {
            order: vec!["id".into(), "name".into()],
            widths: [("id".into(), 82.)].into_iter().collect(),
        });
        let portable = layout.portable();
        assert_eq!(portable.order, ["id", "name"]);
        assert_eq!(portable.widths["id"], 82.);
        assert_eq!(portable.widths["name"], 160.);
        // Only the column the portable layout recorded a width for is pinned;
        // `name` keeps fitting its content.
        assert!(layout.columns[0].exact);
        assert!(!layout.columns[1].exact);
    }

    #[test]
    fn layouts_saved_before_content_fit_do_not_pin_widths() {
        let layout: super::GridLayout =
            serde_json::from_str(r#"{"columns":[{"name":"id","occurrence":0,"width":160.0}]}"#)
                .unwrap();
        assert!(!layout.columns[0].exact);
    }

    #[test]
    fn layouts_round_trip_explicit_widths() {
        let layout: super::GridLayout = serde_json::from_str(
            r#"{"columns":[{"name":"id","occurrence":0,"width":82.0,"exact":true}]}"#,
        )
        .unwrap();
        assert!(layout.columns[0].exact);
        assert_eq!(layout.columns[0].width, 82.);
    }
}
