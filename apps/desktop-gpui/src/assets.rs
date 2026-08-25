use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(icon(path).map(canonical_svg))
    }

    fn list(&self, _: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

/// React's canonical `<I>` wrapper applies rounded caps and joins to every
/// icon. Keep that inherited geometry when the same paths are rendered as
/// standalone GPUI SVG assets.
fn canonical_svg(svg: &'static str) -> Cow<'static, [u8]> {
    let cap = !svg.contains("stroke-linecap=");
    let join = !svg.contains("stroke-linejoin=");
    if !cap && !join {
        return Cow::Borrowed(svg.as_bytes());
    }
    let attributes = match (cap, join) {
        (true, true) => "stroke-linecap=\"round\" stroke-linejoin=\"round\" ",
        (true, false) => "stroke-linecap=\"round\" ",
        (false, true) => "stroke-linejoin=\"round\" ",
        (false, false) => unreachable!(),
    };
    Cow::Owned(
        svg.replacen("<svg ", &format!("<svg {attributes}"), 1)
            .into_bytes(),
    )
}

fn icon(path: &str) -> Option<&'static str> {
    Some(match path {
        "engines/postgres.svg" => include_str!("../../desktop/src/assets/engines/postgres.svg"),
        "engines/firestore.svg" => include_str!("../../desktop/src/assets/engines/firestore.svg"),
        "engines/convex.svg" => include_str!("../../desktop/src/assets/engines/convex.svg"),
        "engines/cosmos.svg" => include_str!("../../desktop/src/assets/engines/cosmos.svg"),
        "engines/mssql.svg" => include_str!("../../desktop/src/assets/engines/mssql.svg"),
        "engines/mysql.svg" => include_str!("../../desktop/src/assets/engines/mysql.svg"),
        "engines/sqlite.svg" => include_str!("../../desktop/src/assets/engines/sqlite.svg"),
        "engines/azure.svg" => include_str!("../../desktop/src/assets/engines/azure.svg"),
        "engines/supabase.svg" => include_str!("../../desktop/src/assets/engines/supabase.svg"),
        "engines/neon.svg" => include_str!("../../desktop/src/assets/engines/neon.svg"),
        "engines/planetscale.svg" => {
            include_str!("../../desktop/src/assets/engines/planetscale.svg")
        }
        "icons/search.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="6"/><path d="M20 20l-4.5-4.5"/></svg>"#
        }
        "icons/filter.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M3 5h18l-7 9v6l-4-2v-4z"/></svg>"#
        }
        "icons/grid-search.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/></svg>"#
        }
        "icons/bookmark.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M6 3h12v18l-6-4.5L6 21z"/></svg>"#
        }
        "icons/check.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M5 12l5 5L20 7"/></svg>"#
        }
        "icons/grid-check.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M4 12l5 5L20 6"/></svg>"#
        }
        "icons/sort-asc.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M7 4v16M3 16l4 4 4-4"/></svg>"#
        }
        "icons/sort-desc.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M7 20V4M3 8l4-4 4 4"/></svg>"#
        }
        "icons/type-key.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><circle cx="8" cy="15" r="4"/><path d="M11 12l8-8M16 7l3 3M14 9l3 3"/></svg>"#
        }
        "icons/type-link.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M10 14L4 20M14 10l6-6M10 6V3h11v11h-3M14 18v3H3V10h3"/></svg>"#
        }
        "icons/type-text.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M4 7V5h16v2M12 5v14M9 19h6"/></svg>"#
        }
        "icons/type-hash.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M4 9h16M4 15h16M10 3L8 21M16 3l-2 18"/></svg>"#
        }
        "icons/type-calendar.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><rect x="3" y="5" width="18" height="16" rx="1.5"/><path d="M3 10h18M8 3v4M16 3v4"/></svg>"#
        }
        "icons/type-bool.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><rect x="2" y="6" width="20" height="12" rx="6"/><circle cx="8" cy="12" r="3" fill="currentColor"/></svg>"#
        }
        "icons/type-json.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M7 4S4 4 4 8s2 4 2 4-2 0-2 4 3 4 3 4M17 4s3 0 3 4-2 4-2 4 2 0 2 4-3 4-3 4"/></svg>"#
        }
        "icons/panel-left.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><rect x="3" y="3" width="18" height="18" rx="1.5"/><path d="M9 3v18"/></svg>"#
        }
        "icons/panel-right.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><rect x="3" y="3" width="18" height="18" rx="1.5"/><path d="M15 3v18"/></svg>"#
        }
        "icons/panel-bottom.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><rect x="3" y="3" width="18" height="18" rx="1.5"/><path d="M3 15h18"/></svg>"#
        }
        "icons/bot.svg" | "icons/asterisk.svg" | "icons/sparkles.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l2 5 5 2-5 2-2 5-2-5-5-2 5-2zM19 13l1 2 2 1-2 1-1 2-1-2-2-1 2-1z"/></svg>"#
        }
        "icons/settings.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9c.34.36.78.59 1.27.65H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>"#
        }
        "icons/edit.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H5a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2v-6M18 2l4 4-10 10H8v-4z"/></svg>"#
        }
        "icons/undo.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M3 8h10a5 5 0 010 10H8M3 8l4-4M3 8l4 4"/></svg>"#
        }
        "icons/download.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M12 3v12M7 10l5 5 5-5M5 21h14"/></svg>"#
        }
        "icons/upload.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M12 21V9M7 14l5-5 5 5M5 3h14"/></svg>"#
        }
        "icons/power.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v10M18.4 6.6a9 9 0 11-12.8 0"/></svg>"#
        }
        "icons/play.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="none"><path d="M7 4.5v15l12-7.5z"/></svg>"#
        }
        "icons/play-small.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="none"><path d="M8 5.5v13l10-6.5z"/></svg>"#
        }
        "icons/stop.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="none"><path d="M6.5 6.5h11v11h-11z"/></svg>"#
        }
        "icons/bolt.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="none"><path d="M13 2L4 14h6l-1 8 9-12h-6z"/></svg>"#
        }
        "icons/copy.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="11" height="11" rx="1.5"/><path d="M5 15H4a1 1 0 01-1-1V4a1 1 0 011-1h10a1 1 0 011 1v1"/></svg>"#
        }
        "icons/format.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round"><path d="M4 6h16M4 10h10M4 14h16M4 18h8"/></svg>"#
        }
        "icons/wrap.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M4 6h16M4 18h6M4 12h14a3 3 0 010 6h-3M12 15l-2 3 2 3"/></svg>"#
        }
        "icons/info.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><circle cx="12" cy="12" r="9"/><path d="M12 16v-4M12 8h.01"/></svg>"#
        }
        "icons/terminal.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M5 8l4 4-4 4M13 16h6"/><rect x="2" y="4" width="20" height="16" rx="1.5"/></svg>"#
        }
        "icons/ssh.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><rect x="2" y="4" width="20" height="16" rx="1.5"/><path d="M6 9l3 3-3 3M13 15h5"/></svg>"#
        }
        "icons/lock.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 018 0v4"/></svg>"#
        }
        "icons/cloud.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M17.5 19a4.5 4.5 0 000-9c-.4-2.8-2.8-5-5.7-5a5.8 5.8 0 00-5.6 4.3A4.4 4.4 0 002 14a4 4 0 004 4z"/></svg>"#
        }
        "icons/eye.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12z"/><circle cx="12" cy="12" r="2.5"/></svg>"#
        }
        "icons/eye-off.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3l18 18M10.6 6.2A10.2 10.2 0 0112 6c6.5 0 10 6 10 6a16.4 16.4 0 01-3.1 3.8M6.4 6.8A16.6 16.6 0 002 12s3.5 6 10 6a9.8 9.8 0 004.1-.9M10.2 10.2a2.5 2.5 0 003.5 3.5"/></svg>"#
        }
        "icons/history.svg" | "icons/book-open.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M3 12a9 9 0 109-9 9 9 0 00-6.4 2.6L3 8M3 3v5h5M12 7v5l3 2"/></svg>"#
        }
        "icons/triangle-alert.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M12 9v4M12 17h.01M10.3 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.41 0z"/></svg>"#
        }
        "icons/layout-dashboard.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><rect x="3" y="4" width="18" height="16" rx="1.5"/><path d="M3 10h18M3 15h18M10 4v16"/></svg>"#
        }
        "icons/table.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="1.5"/><path d="M3 10h18M3 15h18M10 4v16"/></svg>"#
        }
        "icons/database.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="8" ry="2.5"/><path d="M4 5v6c0 1.4 3.6 2.5 8 2.5s8-1.1 8-2.5V5M4 11v6c0 1.4 3.6 2.5 8 2.5s8-1.1 8-2.5v-6"/></svg>"#
        }
        "icons/schema.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h7l2 2h9v11a1 1 0 01-1 1H3a1 1 0 01-1-1V7a1 1 0 011-1z"/></svg>"#
        }
        "icons/folder.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M20 20a2 2 0 002-2V8a2 2 0 00-2-2h-7.9a2 2 0 01-1.69-.9L9.6 3.9A2 2 0 007.93 3H4a2 2 0 00-2 2v13a2 2 0 002 2z"/></svg>"#
        }
        "icons/folder-open.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M6 14l1.45-2.9A2 2 0 019.24 10H20a2 2 0 011.94 2.5l-1.55 6a2 2 0 01-1.94 1.5H4a2 2 0 01-2-2V5a2 2 0 012-2h3.93a2 2 0 011.66.9l.82 1.2a2 2 0 001.66.9H18a2 2 0 012 2v2"/></svg>"#
        }
        "icons/folder-plus.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M20 20a2 2 0 002-2V8a2 2 0 00-2-2h-7.9a2 2 0 01-1.69-.9L9.6 3.9A2 2 0 007.93 3H4a2 2 0 00-2 2v13a2 2 0 002 2z"/><path d="M12 10v6M9 13h6"/></svg>"#
        }
        "icons/trash.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M3 6h18M8 6V4a1 1 0 011-1h6a1 1 0 011 1v2M19 6l-1.4 13.1a2 2 0 01-2 1.9H8.4a2 2 0 01-2-1.9L5 6M10 11v6M14 11v6"/></svg>"#
        }
        "icons/chevron-right.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M9 6l6 6-6 6"/></svg>"#
        }
        "icons/chevron-left.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M15 6l-6 6 6 6"/></svg>"#
        }
        "icons/chevron-down.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M6 9l6 6 6-6"/></svg>"#
        }
        "icons/ellipsis.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/></svg>"#
        }
        "icons/plus.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M12 5v14M5 12h14"/></svg>"#
        }
        "icons/context.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z"/></svg>"#
        }
        "icons/tree.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><circle cx="5" cy="5" r="2"/><circle cx="5" cy="19" r="2"/><circle cx="19" cy="12" r="2"/><path d="M5 7v10M7 5h6.5a3.5 3.5 0 013.5 3.5V12M7 19h6.5a3.5 3.5 0 003.5-3.5V12"/></svg>"#
        }
        "icons/diagram.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><rect x="2.5" y="4" width="8" height="6" rx="1"/><rect x="13.5" y="14" width="8" height="6" rx="1"/><path d="M6.5 10v3a1 1 0 001 1h6"/></svg>"#
        }
        "icons/diff.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v18M9 6L6 9l3 3M15 18l3-3-3-3"/></svg>"#
        }
        "icons/commit.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><circle cx="12" cy="12" r="4"/><path d="M16 12h6M2 12h6"/></svg>"#
        }
        "icons/bracket.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M8 4H4v16h4M16 4h4v16h-4"/></svg>"#
        }
        "icons/paperclip.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M21 11l-9 9a5.5 5.5 0 01-7.8-7.8l9-9a3.7 3.7 0 015.2 5.2l-9 9a1.8 1.8 0 01-2.6-2.6l8-8"/></svg>"#
        }
        "icons/user.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><circle cx="12" cy="8" r="4"/><path d="M4 21v-1a7 7 0 0114 0v1"/></svg>"#
        }
        "icons/send.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M4 12l16-8-6 18-3-7z"/></svg>"#
        }
        "icons/file-text.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8zM14 2v6h6M8 13h8M8 17h5"/></svg>"#
        }
        "icons/expand.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9V3h6M21 9V3h-6M3 15v6h6M21 15v6h-6"/></svg>"#
        }
        "icons/spinner.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="12" cy="12" r="9" opacity="0.2"/><path d="M21 12a9 9 0 00-9-9"/></svg>"#
        }
        "icons/chevrons-down.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M6 6l6 6 6-6M6 13l6 6 6-6"/></svg>"#
        }
        "icons/split-horizontal.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="1.5"/><path d="M3 12h18" stroke-dasharray="2 1.5"/></svg>"#
        }
        "icons/split-vertical.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="1.5"/><path d="M12 3v18" stroke-dasharray="2 1.5"/></svg>"#
        }
        "icons/close.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M6 6l12 12M18 6L6 18"/></svg>"#
        }
        "icons/layout.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="1.5"/><path d="M3 9h18M9 9v12"/></svg>"#
        }
        "icons/star.svg" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l2.6 5.6 6 .7-4.4 4.1 1.2 6L12 16.9 6.6 19.4l1.2-6L3.4 9.3l6-.7z"/></svg>"#
        }
        "icons/gallery-vertical-end.svg" | "icons/cellar-mark.svg" => CELLAR_MARK,
        _ => return None,
    })
}

const CELLAR_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" fill="currentColor"><path d="M502 76C523 73 543 79 563 88L859 261C880 273 892 285 892 301C893 314 886 322 875 328L788 376C781 380 773 380 769 378L564 258C544 247 526 243 511 245C493 247 475 257 451 269L294 357C277 367 267 384 267 401V636C267 655 277 671 294 681L493 796C498 800 499 804 499 808V939C499 946 490 951 480 948L171 769C147 755 132 733 132 715V317C132 295 145 276 169 264L474 87C484 81 494 77 502 76ZM469 326C488 315 496 312 503 312H528C538 314 548 318 556 322L709 409C712 411 712 415 710 417C706 423 702 426 695 430L552 513C541 520 529 522 518 522C506 522 494 519 483 514L336 429C329 425 324 419 322 410L469 326ZM322 465L342 479L479 558C496 568 518 570 531 566L552 559L692 479L707 465C710 464 712 467 712 472L710 528C709 535 704 540 696 544L555 626C539 635 516 637 502 634L475 625L334 542C326 537 322 532 322 528V465ZM322 578L345 594L477 670C494 679 518 682 535 678L556 670L693 590L707 578C710 577 712 581 712 586L710 637C710 646 702 653 692 659L558 737C543 746 519 750 508 747L480 740L334 654C326 649 322 643 322 636V578ZM752 673C764 666 774 661 782 660C789 660 795 660 799 662L880 707C888 712 892 722 892 730C893 740 890 747 886 752C879 760 867 767 855 774L561 945C554 949 547 950 542 946C537 943 535 940 535 936V811C535 801 540 793 549 789L752 673Z"/></svg>"#;

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{canonical_svg, icon};

    #[test]
    fn standalone_assets_keep_the_classic_icon_stroke_geometry() {
        let normalized =
            canonical_svg(r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M1 1h2"/></svg>"#);
        assert!(matches!(normalized, Cow::Owned(_)));
        let normalized = String::from_utf8(normalized.into_owned()).unwrap();
        assert!(normalized.contains("stroke-linecap=\"round\""));
        assert!(normalized.contains("stroke-linejoin=\"round\""));
        assert_eq!(icon("icons/history.svg"), icon("icons/book-open.svg"));
        assert!(icon("icons/check.svg")
            .expect("check icon")
            .contains("M5 12l5 5L20 7"));
        assert!(icon("icons/grid-check.svg")
            .expect("grid check icon")
            .contains("M4 12l5 5L20 6"));
        assert!(icon("icons/chevron-left.svg").is_some());
        assert!(icon("icons/user.svg").is_some());
    }
}
