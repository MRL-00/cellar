//! Cosmos master-key HMAC authorization and HTTP-date helpers.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use cellar_core::error::{CellarError, CellarResult};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

pub(crate) fn master_key_authorization(
    verb: &str,
    resource_type: &str,
    resource_link: &str,
    date: &str,
    key: &[u8],
) -> CellarResult<String> {
    let payload = format!(
        "{}\n{}\n{}\n{}\n\n",
        verb.to_lowercase(),
        resource_type.to_lowercase(),
        resource_link,
        date.to_lowercase()
    );
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| CellarError::invalid_config(format!("invalid Cosmos key: {e}")))?;
    mac.update(payload.as_bytes());
    let signature = BASE64.encode(mac.finalize().into_bytes());
    let token = format!("type=master&ver=1.0&sig={signature}");
    Ok(urlencoding::encode(&token).into_owned())
}

pub(crate) fn rfc1123_now() -> String {
    use std::time::Duration as StdDuration;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(StdDuration::ZERO)
        .as_secs();
    httpdate_from_unix(now)
}

pub(crate) fn httpdate_from_unix(secs: u64) -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let day_secs = secs % 86_400;
    let days = secs / 86_400;
    let weekday = DAYS[(days % 7) as usize];
    let (year, month, day) = civil_from_days(days as i64);
    let hour = day_secs / 3600;
    let minute = (day_secs % 3600) / 60;
    let second = day_secs % 60;
    format!(
        "{weekday}, {day:02} {month} {year:04} {hour:02}:{minute:02}:{second:02} GMT",
        month = MONTHS[month as usize]
    )
}

/// Convert days since Unix epoch to (year, month 0-11, day 1-31).
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    // Algorithm from Howard Hinnant / civil_from_days.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, (m - 1) as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_signature_is_stable_for_known_payload() {
        let key = [0u8; 64];
        let date = "Tue, 01 Jan 2019 00:00:00 GMT";
        let auth = master_key_authorization("GET", "dbs", "", date, &key).expect("auth");
        assert!(auth.starts_with("type%3Dmaster%26ver%3D1.0%26sig%3D"));
        let auth2 = master_key_authorization("GET", "dbs", "", date, &key).expect("auth");
        assert_eq!(auth, auth2);

        let docs = master_key_authorization(
            "POST",
            "docs",
            "dbs/mydb/colls/mycoll",
            date,
            &key,
        )
        .expect("docs auth");
        assert_ne!(auth, docs);
    }

    #[test]
    fn httpdate_formats_unix_epoch_thursday() {
        assert_eq!(httpdate_from_unix(0), "Thu, 01 Jan 1970 00:00:00 GMT");
        assert_eq!(
            httpdate_from_unix(1_546_300_800),
            "Tue, 01 Jan 2019 00:00:00 GMT"
        );
    }
}
