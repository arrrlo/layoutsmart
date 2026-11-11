//! Timestamps without a date library: Unix seconds → ISO 8601 (UTC).

const SECS_PER_DAY: u64 = 86_400;

/// `1794390124` → `"2026-11-11T09:42:04Z"`.
pub fn iso8601(unix_secs: u64) -> String {
    let (year, month, day) = civil_from_days(unix_secs / SECS_PER_DAY);
    let secs = unix_secs % SECS_PER_DAY;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// Days since 1970-01-01 → (year, month, day). Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

/// `"2026-11-11T09:42:04Z"` → `"2026-11-11 09:42 UTC"`. Anything else is returned unchanged.
pub fn display(iso: &str) -> String {
    let b = iso.as_bytes();
    if iso.is_ascii() && b.len() == 20 && b[10] == b'T' && b[19] == b'Z' {
        format!("{} {} UTC", &iso[..10], &iso[11..16])
    } else {
        iso.to_string()
    }
}

#[cfg(test)]
mod tests;
