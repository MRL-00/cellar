use cellar_core::value::{CellValue, ColumnMeta};
use gpui::{div, prelude::*, px, AnyElement, ClipboardItem, SharedString};
use gpui_component::{
    button::{Button, ButtonVariants},
    popover::Popover,
    scroll::ScrollableElement,
    IconName,
};

use crate::theme::{
    ACCENT, BORDER, BORDER_DIVIDER, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL, PANEL_RAISED,
};

#[derive(Clone)]
enum ExpandedValue {
    Json(String),
    Array(Vec<String>),
    Bytes(Vec<u8>),
    Geometry(String),
}

pub(super) fn rich_cell_content(
    row: usize,
    column_index: usize,
    selected: bool,
    column: Option<&ColumnMeta>,
    value: Option<&CellValue>,
    fallback: String,
) -> AnyElement {
    let data_type = column
        .map(|column| column.data_type.to_ascii_lowercase())
        .unwrap_or_default();
    let title = column
        .map(|column| format!("{} · {}", column.name, column.data_type))
        .unwrap_or_default();
    let (inline, raw, expanded) = match value {
        Some(CellValue::Json(value)) => {
            let raw = value.to_string();
            let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| raw.clone());
            (
                div()
                    .truncate()
                    .text_color(FG_MUTED)
                    .child(json_summary(value))
                    .into_any_element(),
                raw,
                ExpandedValue::Json(pretty),
            )
        }
        Some(CellValue::Bytes(bytes)) => {
            let head = bytes
                .iter()
                .take(8)
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let raw = format!(
                "\\x{}",
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
            (
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .font_family(crate::theme::mono_font())
                    .child(format!(
                        "\\x{head}{}",
                        if bytes.len() > 8 { "…" } else { "" }
                    ))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(11.))
                            .text_color(FG_MUTED)
                            .child(format_byte_size(bytes.len())),
                    )
                    .into_any_element(),
                raw,
                ExpandedValue::Bytes(bytes.clone()),
            )
        }
        Some(CellValue::Text(text)) if is_array_type(&data_type) => {
            let values = parse_pg_array(text);
            let inline = array_inline(&values, text);
            (inline, text.clone(), ExpandedValue::Array(values))
        }
        Some(CellValue::Text(text)) if is_geometry_type(&data_type) => (
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(div().text_color(ACCENT).child("◈"))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(FG_SECONDARY)
                        .child(geometry_label(text)),
                )
                .into_any_element(),
            text.clone(),
            ExpandedValue::Geometry(text.clone()),
        ),
        _ => return div().truncate().child(fallback).into_any_element(),
    };

    let popover_id = SharedString::from(format!("rich-cell:{row}:{column_index}"));
    let button_id = SharedString::from(format!("rich-cell-expand:{row}:{column_index}"));
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(4.))
        .child(div().min_w_0().truncate().child(inline))
        .child(
            Popover::new(popover_id)
                .appearance(false)
                .trigger(
                    Button::new(button_id)
                        .icon(IconName::Maximize)
                        .tooltip("Expand cell")
                        .compact()
                        .ghost()
                        .size(px(15.))
                        .opacity(if selected { 1. } else { 0. })
                        .group_hover("grid-cell", |style| style.opacity(1.))
                        .on_click(|_, _, cx| cx.stop_propagation()),
                )
                .content(move |_, _, _| rich_popover(title.clone(), raw.clone(), expanded.clone())),
        )
        .into_any_element()
}

fn rich_popover(title: String, raw: String, value: ExpandedValue) -> impl IntoElement {
    let copy = raw.clone();
    div()
        .w(px(420.))
        .max_h(px(420.))
        .flex()
        .flex_col()
        .rounded(px(6.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL)
        .shadow_lg()
        .overflow_hidden()
        .text_size(px(12.))
        .text_color(FG_SECONDARY)
        .child(
            div()
                .h(px(28.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .px_2()
                .border_b_1()
                .border_color(BORDER_DIVIDER)
                .bg(PANEL_RAISED)
                .text_size(px(11.))
                .text_color(FG_MUTED)
                .child(div().min_w_0().truncate().child(title.to_lowercase()))
                .child(
                    Button::new("copy-rich-cell")
                        .label("Copy")
                        .compact()
                        .outline()
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()))
                        }),
                ),
        )
        .child(
            div()
                .min_h_0()
                .overflow_y_scrollbar()
                .p_2()
                .child(expanded_content(value)),
        )
}

fn expanded_content(value: ExpandedValue) -> AnyElement {
    match value {
        ExpandedValue::Json(pretty) => div()
            .font_family(crate::theme::mono_font())
            .line_height(px(18.))
            .child(pretty)
            .into_any_element(),
        ExpandedValue::Array(values) => div()
            .flex()
            .flex_col()
            .gap(px(3.))
            .children(values.into_iter().enumerate().map(|(index, value)| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .w(px(24.))
                            .text_align(gpui::TextAlign::Right)
                            .text_size(px(11.))
                            .text_color(FG_MUTED)
                            .child(index.to_string()),
                    )
                    .child(chip(value))
            }))
            .into_any_element(),
        ExpandedValue::Bytes(bytes) => div()
            .font_family(crate::theme::mono_font())
            .line_height(px(18.))
            .children(hex_dump(&bytes).into_iter().map(|line| div().child(line)))
            .into_any_element(),
        ExpandedValue::Geometry(raw) => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .h(px(96.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(INSET)
                    .text_size(px(11.))
                    .text_color(FG_MUTED)
                    .child("Map preview not available yet"),
            )
            .child(
                div()
                    .font_family(crate::theme::mono_font())
                    .text_color(FG)
                    .child(raw),
            )
            .into_any_element(),
    }
}

fn array_inline(values: &[String], raw: &str) -> AnyElement {
    if values.is_empty() {
        return div()
            .text_color(FG_MUTED)
            .child(if raw.trim() == "{}" {
                "{ }".to_owned()
            } else {
                raw.to_owned()
            })
            .into_any_element();
    }
    let overflow = values.len().saturating_sub(8);
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(3.))
        .children(values.iter().take(8).cloned().map(chip))
        .when(overflow > 0, |element| {
            element.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(11.))
                    .text_color(FG_MUTED)
                    .child(format!("+{overflow}")),
            )
        })
        .into_any_element()
}

fn chip(value: String) -> impl IntoElement {
    div()
        .max_w(px(140.))
        .truncate()
        .px(px(5.))
        .rounded(px(3.))
        .border_1()
        .border_color(BORDER_DIVIDER)
        .bg(PANEL_RAISED)
        .text_size(px(11.))
        .child(value)
}

fn json_summary(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Array(values) => match values.len() {
            0 => "[ ]".into(),
            1 => "[ 1 item ]".into(),
            count => format!("[ {count} items ]"),
        },
        serde_json::Value::Object(values) => match values.len() {
            0 => "{ }".into(),
            1 => "{ 1 key }".into(),
            count => format!("{{ {count} keys }}"),
        },
        value => value.to_string(),
    }
}

fn is_array_type(data_type: &str) -> bool {
    data_type.ends_with("[]")
        || data_type.ends_with(" array")
        || data_type.starts_with('_') && data_type.len() > 1
}

fn parse_pg_array(literal: &str) -> Vec<String> {
    let Some(inner) = literal
        .trim()
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
    else {
        return Vec::new();
    };
    if inner.is_empty() {
        return Vec::new();
    }
    let mut values = Vec::new();
    let (mut current, mut depth, mut quoted, mut escaped) = (String::new(), 0usize, false, false);
    for character in inner.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if quoted && character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if !quoted && character == '{' {
            depth += 1;
            current.push(character);
        } else if !quoted && character == '}' {
            depth = depth.saturating_sub(1);
            current.push(character);
        } else if !quoted && depth == 0 && character == ',' {
            values.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(character);
        }
    }
    values.push(current.trim().to_owned());
    values
}

fn format_byte_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024. * 1024.))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024. * 1024. * 1024.))
    }
}

fn hex_dump(bytes: &[u8]) -> Vec<String> {
    bytes
        .iter()
        .take(256)
        .collect::<Vec<_>>()
        .chunks(16)
        .enumerate()
        .map(|(row, chunk)| {
            let hex = chunk
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            let ascii = chunk
                .iter()
                .map(|byte| {
                    if byte.is_ascii_graphic() {
                        char::from(**byte)
                    } else {
                        '.'
                    }
                })
                .collect::<String>();
            format!("{:08x}  {:47}  {ascii}", row * 16, hex)
        })
        .collect()
}

fn is_geometry_type(data_type: &str) -> bool {
    data_type.starts_with("geometry") || data_type.starts_with("geography")
}

fn geometry_label(value: &str) -> String {
    value
        .trim()
        .strip_prefix("SRID=")
        .and_then(|value| value.split_once(';').map(|(_, geometry)| geometry))
        .unwrap_or(value.trim())
        .chars()
        .take_while(|character| character.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::{format_byte_size, geometry_label, hex_dump, json_summary, parse_pg_array};

    #[test]
    fn rich_grid_summaries_and_expansions_match_the_classic_renderers() {
        assert_eq!(
            json_summary(&serde_json::json!({"a": 1, "b": 2})),
            "{ 2 keys }"
        );
        assert_eq!(json_summary(&serde_json::json!([1])), "[ 1 item ]");
        assert_eq!(
            parse_pg_array(r#"{1,"b,c",{2,3},NULL}"#),
            ["1", "b,c", "{2,3}", "NULL"]
        );
        assert_eq!(format_byte_size(1536), "1.5 KB");
        assert_eq!(geometry_label("SRID=4326;POINT(1 2)"), "POINT");
        assert_eq!(
            hex_dump(b"Cellar")[0],
            "00000000  43 65 6c 6c 61 72                                Cellar"
        );
    }
}
