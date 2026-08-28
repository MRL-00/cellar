#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqlStatement<'a> {
    pub text: &'a str,
    pub start: usize,
    pub end: usize,
    pub raw_start: usize,
    pub raw_end: usize,
    pub start_line: usize,
    pub end_line: usize,
}

pub fn split_statements(sql: &str) -> Vec<SqlStatement<'_>> {
    partition(sql)
        .into_iter()
        .filter(|statement| !statement.text.is_empty())
        .collect()
}

pub fn statement_at_offset(sql: &str, offset: usize) -> Option<SqlStatement<'_>> {
    let chunks = partition(sql);
    let offset = offset.min(sql.len());
    if let Some(statement) = chunks
        .iter()
        .find(|statement| offset >= statement.raw_start && offset < statement.raw_end)
        .filter(|statement| !statement.text.is_empty())
    {
        return Some(*statement);
    }
    let non_empty = chunks
        .into_iter()
        .filter(|statement| !statement.text.is_empty())
        .collect::<Vec<_>>();
    non_empty
        .iter()
        .rev()
        .find(|statement| statement.raw_start <= offset)
        .copied()
        .or_else(|| non_empty.first().copied())
}

fn partition(sql: &str) -> Vec<SqlStatement<'_>> {
    let mut chunks = Vec::new();
    let mut start = 0;
    for cut in top_level_semicolons(sql) {
        chunks.push(make_statement(sql, start, cut + 1));
        start = cut + 1;
    }
    if start < sql.len() || chunks.is_empty() {
        chunks.push(make_statement(sql, start, sql.len()));
    }
    chunks
}

fn top_level_semicolons(sql: &str) -> Vec<usize> {
    let bytes = sql.as_bytes();
    let mut cuts = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => index = skip_quoted(bytes, index, bytes[index]),
            b'-' if bytes.get(index + 1) == Some(&b'-') => {
                index = bytes[index + 2..]
                    .iter()
                    .position(|byte| *byte == b'\n')
                    .map_or(bytes.len(), |next| index + 2 + next);
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => index = skip_block_comment(bytes, index),
            b'$' => {
                if let Some(next) = skip_dollar_quote(sql, index) {
                    index = next;
                } else {
                    index += 1;
                }
            }
            b';' => {
                cuts.push(index);
                index += 1;
            }
            _ => index += 1,
        }
    }
    cuts
}

fn skip_quoted(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut index = start + 1;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index = (index + 2).min(bytes.len());
        } else if bytes[index] == quote {
            if bytes.get(index + 1) == Some(&quote) {
                index += 2;
            } else {
                return index + 1;
            }
        } else {
            index += 1;
        }
    }
    bytes.len()
}

fn skip_block_comment(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 2;
    while index + 1 < bytes.len() {
        if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            return index + 2;
        }
        index += 1;
    }
    bytes.len()
}

fn skip_dollar_quote(sql: &str, start: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let mut end = start + 1;
    if bytes
        .get(end)
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
    {
        end += 1;
        while bytes
            .get(end)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            end += 1;
        }
    }
    if bytes.get(end) != Some(&b'$') {
        return None;
    }
    let tag = &sql[start..=end];
    sql[end + 1..]
        .find(tag)
        .map_or(Some(sql.len()), |close| Some(end + 1 + close + tag.len()))
}

fn make_statement(sql: &str, raw_start: usize, raw_end: usize) -> SqlStatement<'_> {
    let mut start = raw_start;
    let mut end = raw_end;
    if end > start && sql.as_bytes()[end - 1] == b';' {
        end -= 1;
    }
    while start < end && sql.as_bytes()[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && sql.as_bytes()[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    SqlStatement {
        text: &sql[start..end],
        start,
        end,
        raw_start,
        raw_end,
        start_line: line_at(sql, start),
        end_line: line_at(sql, end.saturating_sub(1).max(start)),
    }
}

fn line_at(sql: &str, offset: usize) -> usize {
    1 + sql.as_bytes()[..offset.min(sql.len())]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
}

#[cfg(test)]
mod tests {
    use super::{split_statements, statement_at_offset};

    #[test]
    fn selects_statement_under_cursor_and_ignores_nested_semicolons() {
        let sql = "SELECT ';';\n-- ;\nSELECT $$x;y$$;\nSELECT 3";
        let statements = split_statements(sql);
        assert_eq!(
            statements.iter().map(|s| s.text).collect::<Vec<_>>(),
            vec!["SELECT ';'", "-- ;\nSELECT $$x;y$$", "SELECT 3",]
        );
        assert_eq!(
            statement_at_offset(sql, sql.len()).unwrap().text,
            "SELECT 3"
        );
        assert_eq!(
            statement_at_offset("SELECT 1;\n\nSELECT 2", 10)
                .unwrap()
                .text,
            "SELECT 2"
        );
    }
}
