//! Serializes JSON cell values into display text plus syntax-highlight ranges.
//!
//! Output is capped at a byte limit so a large document never produces an
//! unbounded layout inside a grid cell or popover.

use std::ops::Range;

use gpui::{App, HighlightStyle, StyledText};
use gpui_component::Theme as ComponentTheme;
use serde_json::Value;

use crate::theme::{syntax_keyword, FG, FG_TERTIARY};

/// Token colors. Strings and numbers come from the SQL editor's highlight
/// theme so JSON and SQL share one palette; keys and punctuation use Cellar's
/// foreground tokens so structure reads by brightness.
#[derive(Clone, Copy)]
pub(super) struct JsonPalette {
    key: HighlightStyle,
    string: HighlightStyle,
    number: HighlightStyle,
    literal: HighlightStyle,
    punctuation: HighlightStyle,
}

impl JsonPalette {
    /// Headless harnesses such as `cellar-grid-perf` render the grid without
    /// `gpui_component::init`, so a missing theme falls back to plain tokens.
    pub fn from_theme(cx: &App) -> Self {
        let syntax = cx
            .try_global::<ComponentTheme>()
            .map(|theme| &theme.highlight_theme.style.syntax);
        let token = |name: &str| syntax.and_then(|syntax| syntax.style(name));
        let color = |color: crate::theme::DynamicColor| HighlightStyle {
            color: Some(color.into()),
            ..Default::default()
        };
        let fallback = color(FG);
        Self {
            key: color(FG),
            string: token("string").unwrap_or(fallback),
            number: token("number").unwrap_or(fallback),
            // The stock highlight themes paint booleans the same as numbers;
            // Cellar's keyword tint keeps `true`/`false`/`null` distinct.
            literal: HighlightStyle {
                color: Some(syntax_keyword(1.).into()),
                ..Default::default()
            },
            punctuation: color(FG_TERTIARY),
        }
    }

    fn style(&self, token: JsonToken) -> HighlightStyle {
        match token {
            JsonToken::Key => self.key,
            JsonToken::String => self.string,
            JsonToken::Number => self.number,
            JsonToken::Literal => self.literal,
            JsonToken::Punctuation => self.punctuation,
        }
    }
}

impl HighlightedJson {
    pub fn styled(self, palette: JsonPalette) -> StyledText {
        let highlights = self
            .spans
            .into_iter()
            .map(|(range, token)| (range, palette.style(token)))
            .collect::<Vec<_>>();
        StyledText::new(self.text).with_highlights(highlights)
    }

    /// Splits pretty output into one styled line each. The expanded popover
    /// renders inside a grid cell and inherits its single-line truncation, so
    /// multi-line text has to be laid out as separate line elements.
    pub fn styled_lines(self, palette: JsonPalette) -> Vec<StyledText> {
        self.lines()
            .into_iter()
            .map(|(line, spans)| {
                let highlights = spans
                    .into_iter()
                    .map(|(range, token)| (range, palette.style(token)))
                    .collect::<Vec<_>>();
                StyledText::new(line).with_highlights(highlights)
            })
            .collect()
    }

    /// Lines with spans rebased to each line's start. Tokens never cross a
    /// newline because JSON strings escape them.
    fn lines(self) -> Vec<(String, Vec<(Range<usize>, JsonToken)>)> {
        let mut spans = self.spans.into_iter().peekable();
        let mut start = 0;
        self.text
            .split('\n')
            .map(|line| {
                let end = start + line.len();
                let mut line_spans = Vec::new();
                while let Some((range, token)) = spans.next_if(|(range, _)| range.end <= end) {
                    line_spans.push((range.start - start..range.end - start, token));
                }
                start = end + 1;
                (line.to_owned(), line_spans)
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum JsonToken {
    Key,
    String,
    Number,
    Literal,
    Punctuation,
}

#[derive(Clone, Copy)]
pub(super) enum JsonLayout {
    /// One line, `{"a": 1, "b": [2, 3]}`, for grid cells.
    Inline,
    /// Two-space indentation matching `serde_json::to_string_pretty`.
    Pretty,
}

#[derive(Clone, Debug, Default)]
pub(super) struct HighlightedJson {
    pub text: String,
    pub spans: Vec<(Range<usize>, JsonToken)>,
    pub truncated: bool,
}

pub(super) fn highlight_json(value: &Value, layout: JsonLayout, limit: usize) -> HighlightedJson {
    let mut writer = Writer {
        out: HighlightedJson::default(),
        limit,
    };
    writer.value(value, layout, 0);
    if writer.out.truncated {
        writer.out.text.push('…');
    }
    writer.out
}

struct Writer {
    out: HighlightedJson,
    limit: usize,
}

impl Writer {
    fn value(&mut self, value: &Value, layout: JsonLayout, depth: usize) {
        match value {
            Value::Null | Value::Bool(_) => self.push(JsonToken::Literal, &value.to_string()),
            Value::Number(number) => self.push(JsonToken::Number, &number.to_string()),
            Value::String(_) => self.push(JsonToken::String, &value.to_string()),
            Value::Array(items) => {
                self.push(JsonToken::Punctuation, "[");
                for (index, item) in items.iter().enumerate() {
                    if self.out.truncated {
                        return;
                    }
                    self.separator(index, layout, depth);
                    self.value(item, layout, depth + 1);
                }
                self.close("]", !items.is_empty(), layout, depth);
            }
            Value::Object(entries) => {
                self.push(JsonToken::Punctuation, "{");
                for (index, (key, item)) in entries.iter().enumerate() {
                    if self.out.truncated {
                        return;
                    }
                    self.separator(index, layout, depth);
                    self.push(JsonToken::Key, &Value::String(key.clone()).to_string());
                    self.push(JsonToken::Punctuation, ":");
                    self.push_plain(" ");
                    self.value(item, layout, depth + 1);
                }
                self.close("}", !entries.is_empty(), layout, depth);
            }
        }
    }

    fn separator(&mut self, index: usize, layout: JsonLayout, depth: usize) {
        if index > 0 {
            self.push(JsonToken::Punctuation, ",");
            if matches!(layout, JsonLayout::Inline) {
                self.push_plain(" ");
            }
        }
        if matches!(layout, JsonLayout::Pretty) {
            self.newline(depth + 1);
        }
    }

    fn close(&mut self, bracket: &str, non_empty: bool, layout: JsonLayout, depth: usize) {
        if non_empty && matches!(layout, JsonLayout::Pretty) {
            self.newline(depth);
        }
        self.push(JsonToken::Punctuation, bracket);
    }

    fn newline(&mut self, depth: usize) {
        self.push_plain(&format!("\n{}", "  ".repeat(depth)));
    }

    fn push_plain(&mut self, text: &str) {
        self.append(text);
    }

    fn push(&mut self, token: JsonToken, text: &str) {
        let start = self.out.text.len();
        self.append(text);
        let end = self.out.text.len();
        if end > start {
            self.out.spans.push((start..end, token));
        }
    }

    fn append(&mut self, text: &str) {
        if self.out.truncated {
            return;
        }
        let room = self.limit.saturating_sub(self.out.text.len());
        if text.len() <= room {
            self.out.text.push_str(text);
            return;
        }
        let mut cut = room;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        self.out.text.push_str(&text[..cut]);
        self.out.truncated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::{highlight_json, JsonLayout, JsonToken};
    use serde_json::json;

    fn tokens(value: &serde_json::Value, layout: JsonLayout) -> Vec<(String, JsonToken)> {
        let output = highlight_json(value, layout, usize::MAX);
        output
            .spans
            .iter()
            .map(|(range, token)| (output.text[range.clone()].to_owned(), *token))
            .collect()
    }

    #[test]
    fn inline_layout_is_single_line_with_readable_spacing() {
        let value = json!({"fps": 30, "on": true, "tags": ["a", null]});
        let output = highlight_json(&value, JsonLayout::Inline, usize::MAX);
        assert_eq!(
            output.text,
            r#"{"fps": 30, "on": true, "tags": ["a", null]}"#
        );
        assert!(!output.truncated);
    }

    #[test]
    fn pretty_layout_matches_serde_pretty_printer() {
        let value = json!({"a": [1, {"b": "x", "c": []}], "d": {}, "e": {"f": null}});
        let output = highlight_json(&value, JsonLayout::Pretty, usize::MAX);
        assert_eq!(output.text, serde_json::to_string_pretty(&value).unwrap());
    }

    #[test]
    fn keys_and_string_values_get_distinct_tokens_with_escapes_intact() {
        // Keys are already sorted so the test holds with or without
        // serde_json's `preserve_order` feature.
        let value = json!({"n": -1.5, "ok": false, "say \"hi\"": "v\n"});
        assert_eq!(
            tokens(&value, JsonLayout::Inline),
            [
                ("{".into(), JsonToken::Punctuation),
                ("\"n\"".into(), JsonToken::Key),
                (":".into(), JsonToken::Punctuation),
                ("-1.5".into(), JsonToken::Number),
                (",".into(), JsonToken::Punctuation),
                ("\"ok\"".into(), JsonToken::Key),
                (":".into(), JsonToken::Punctuation),
                ("false".into(), JsonToken::Literal),
                (",".into(), JsonToken::Punctuation),
                (r#""say \"hi\"""#.into(), JsonToken::Key),
                (":".into(), JsonToken::Punctuation),
                (r#""v\n""#.into(), JsonToken::String),
                ("}".into(), JsonToken::Punctuation),
            ]
        );
    }

    #[test]
    fn truncation_respects_limit_and_utf8_boundaries() {
        // `{"k": "é` is 9 bytes; a 9-byte limit would split the second `é`.
        let value = json!({"k": "ééé"});
        let output = highlight_json(&value, JsonLayout::Inline, 9);
        assert!(output.truncated);
        assert_eq!(output.text, "{\"k\": \"é…");
        let last = output.spans.last().unwrap();
        assert_eq!(&output.text[last.0.clone()], "\"é");
        assert_eq!(last.1, JsonToken::String);
    }

    #[test]
    fn pretty_lines_rebase_spans_to_each_line() {
        let value = json!({"a": [1, "x"]});
        let lines = highlight_json(&value, JsonLayout::Pretty, usize::MAX).lines();
        let rendered = lines
            .iter()
            .map(|(line, spans)| {
                spans
                    .iter()
                    .map(|(range, token)| format!("{:?}:{}", token, &line[range.clone()]))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            lines
                .iter()
                .map(|(line, _)| line.as_str())
                .collect::<Vec<_>>(),
            ["{", "  \"a\": [", "    1,", "    \"x\"", "  ]", "}"]
        );
        assert_eq!(
            rendered,
            [
                "Punctuation:{",
                "Key:\"a\" Punctuation:: Punctuation:[",
                "Number:1 Punctuation:,",
                "String:\"x\"",
                "Punctuation:]",
                "Punctuation:}",
            ]
        );
    }

    #[test]
    fn truncation_stops_walking_large_arrays() {
        let value = serde_json::Value::Array((0..100_000).map(|n| json!(n)).collect());
        let output = highlight_json(&value, JsonLayout::Inline, 64);
        assert!(output.truncated);
        assert!(output.text.len() <= 64 + '…'.len_utf8());
    }
}
