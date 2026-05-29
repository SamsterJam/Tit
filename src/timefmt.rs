//! Single source of truth for all datetime and duration formatting/parsing.

use chrono::{Duration, Local, NaiveDateTime};

use crate::error::TitError;

/// On-disk storage format (matches the Python tool: ISO, no microseconds/tz).
const STORAGE: &str = "%Y-%m-%dT%H:%M:%S";
/// Compact display format, e.g. `2024-05-21 11:36 AM`.
const DISPLAY: &str = "%Y-%m-%d %I:%M %p";
/// Git-style commit date, e.g. `Tue May 21 11:36:05 2024`.
const COMMIT_DATE: &str = "%a %b %d %H:%M:%S %Y";

/// Current local time as a naive datetime (matches Python `datetime.now()`).
pub fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

/// Parse a stored timestamp. Tolerant of fractional seconds that legacy data
/// may carry.
pub fn parse(s: &str) -> Result<NaiveDateTime, TitError> {
    NaiveDateTime::parse_from_str(s, STORAGE)
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
        .or_else(|_| s.parse::<NaiveDateTime>())
        .map_err(|_| TitError::Parse(s.to_string()))
}

pub fn to_storage(dt: NaiveDateTime) -> String {
    dt.format(STORAGE).to_string()
}

pub fn display(dt: NaiveDateTime) -> String {
    dt.format(DISPLAY).to_string()
}

pub fn commit_date(dt: NaiveDateTime) -> String {
    dt.format(COMMIT_DATE).to_string()
}

/// Split a duration into its `(hours, minutes, seconds)` parts. Hours can
/// exceed 24 (never abbreviated to days).
fn hms_parts(d: Duration) -> (i64, i64, i64) {
    let total = d.num_seconds().max(0);
    (total / 3600, (total % 3600) / 60, total % 60)
}

/// Default duration format `H:MM:SS` (unpadded hours), used by `log`, `time`,
/// `today`, `status`, and session listings — matching the original tool.
pub fn duration_hms(d: Duration) -> String {
    let (h, m, s) = hms_parts(d);
    format!("{h}:{m:02}:{s:02}")
}

/// Zero-padded duration `HH:MM:SS`, used by the `export` table.
pub fn duration_hms_padded(d: Duration) -> String {
    let (h, m, s) = hms_parts(d);
    format!("{h:02}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_storage_roundtrip() {
        let dt = parse("2024-05-21T11:36:05").unwrap();
        assert_eq!(to_storage(dt), "2024-05-21T11:36:05");
    }

    #[test]
    fn parse_tolerates_fractional() {
        assert!(parse("2024-05-21T11:36:05.123456").is_ok());
    }

    #[test]
    fn duration_does_not_abbreviate_over_24h() {
        let d = Duration::seconds(313 * 3600 + 12 * 60 + 34);
        assert_eq!(duration_hms(d), "313:12:34");
        assert_eq!(duration_hms_padded(d), "313:12:34");
    }

    #[test]
    fn duration_hours_are_unpadded() {
        assert_eq!(
            duration_hms(Duration::seconds(3 * 3600 + 8 * 60 + 24)),
            "3:08:24"
        );
        assert_eq!(duration_hms(Duration::seconds(5)), "0:00:05");
    }

    #[test]
    fn padded_form_zero_pads_hours() {
        assert_eq!(
            duration_hms_padded(Duration::seconds(3 * 3600 + 8 * 60 + 24)),
            "03:08:24"
        );
        assert_eq!(duration_hms_padded(Duration::seconds(5)), "00:00:05");
    }
}
