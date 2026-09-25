use filiz::ui::format::{bytes, percent, rate, uptime};

#[test]
fn bytes_scale_through_binary_units() {
    assert_eq!(bytes(0), "0.0B");
    assert_eq!(bytes(1536), "1.5KB");
    assert_eq!(bytes(5 * 1024 * 1024 * 1024), "5.0GB");
}

#[test]
fn rate_clamps_negative_values_and_appends_per_second() {
    assert_eq!(rate(2048.0), "2.0KB/s");
    assert_eq!(rate(-5.0), "0.0B/s");
}

#[test]
fn percent_renders_missing_values_as_na() {
    assert_eq!(percent(Some(61.6)), "62%");
    assert_eq!(percent(None), "N/A");
}

#[test]
fn uptime_switches_to_days_after_24_hours() {
    assert_eq!(uptime(3_700), "1h 1m");
    assert_eq!(uptime(90_061), "1d 1h");
}
