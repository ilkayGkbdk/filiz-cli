#[allow(dead_code)]
#[path = "../src/collectors/mod.rs"]
mod collectors;
#[allow(dead_code)]
#[path = "../src/model.rs"]
mod model;

use collectors::macos::{parse_battery, MacOsCollector};
use collectors::system::{byte_rate, percentage};
use collectors::Collector;

#[test]
fn byte_rate_uses_elapsed_seconds_between_fixed_samples() {
    assert_eq!(byte_rate(1_000, 3_500, 2.5), Some(1_000.0));
    assert_eq!(byte_rate(3_500, 4_000, 0.5), Some(1_000.0));
}

#[test]
fn byte_rate_rejects_reset_counters_and_zero_interval() {
    assert_eq!(byte_rate(3_500, 100, 1.0), None);
    assert_eq!(byte_rate(1_000, 3_500, 0.0), None);
}

#[test]
fn percentage_converts_two_fixed_samples() {
    assert_eq!(percentage(25, 100), Some(25.0));
    assert_eq!(percentage(75, 200), Some(37.5));
    assert_eq!(percentage(0, 0), None);
}

#[test]
fn collector_set_returns_usable_snapshot() {
    let mut collectors = collectors::CollectorSet::new();
    let snapshot = collectors.snapshot();
    assert!(snapshot
        .metrics
        .iter()
        .any(|metric| metric.name == "uptime"));
    assert!(snapshot
        .metrics
        .iter()
        .any(|metric| metric.name == "memory.used"));
}

#[test]
fn process_collector_preserves_start_time_and_pid_order() {
    let mut collector = collectors::processes::ProcessCollector::new();
    let result = collector.collect().expect("process collection");
    assert!(result
        .processes
        .windows(2)
        .all(|pair| pair[0].identity.pid < pair[1].identity.pid));
    let current_pid = sysinfo::get_current_pid().expect("current PID").as_u32();
    let current = result
        .processes
        .iter()
        .find(|process| process.identity.pid == current_pid)
        .expect("current process is present");
    assert!(current.identity.start_time > 0);
}

#[test]
fn process_cpu_is_unavailable_until_a_second_refresh() {
    let mut collector = collectors::processes::ProcessCollector::new();
    let first = collector.collect().expect("first collection");
    let current_pid = sysinfo::get_current_pid().expect("current PID").as_u32();
    let first_current = first
        .processes
        .iter()
        .find(|process| process.identity.pid == current_pid)
        .expect("current process is present");
    assert_eq!(first_current.cpu_percent, None);

    let second = collector.collect().expect("second collection");
    let second_current = second
        .processes
        .iter()
        .find(|process| process.identity.pid == current_pid)
        .expect("current process is present");
    assert!(second_current.cpu_percent.is_some());
}

#[test]
fn macos_battery_parser_extracts_typed_values() {
    let sample = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1234567)\t78%; discharging; 3:20 remaining present: true";
    let battery = parse_battery(sample).expect("battery values");
    assert_eq!(battery.percent, 78.0);
    assert_eq!(battery.power_source.as_deref(), Some("Battery Power"));
    assert_eq!(battery.charging, Some(false));
}

#[test]
fn macos_battery_parser_rejects_unexposed_percentage() {
    assert!(parse_battery("Now drawing from 'AC Power'\n No batteries available").is_none());
}

#[test]
fn macos_battery_parser_keeps_percent_when_status_is_unavailable() {
    let sample = "Now drawing from 'Battery Power'\n -InternalBattery-0\t78%; unknown;";
    let battery = parse_battery(sample).expect("battery percent");
    assert_eq!(battery.percent, 78.0);
    assert_eq!(battery.charging, None);
}

#[test]
fn macos_collector_reports_unavailable_fields_with_warnings() {
    let values = MacOsCollector::new().collect();
    if values.battery_percent.is_none() {
        assert!(values
            .warnings
            .iter()
            .any(|warning| warning.collector == "battery"));
    }
    if values.temperature_celsius.is_none() {
        assert!(values
            .warnings
            .iter()
            .any(|warning| warning.collector == "temperature"));
    }
}

#[test]
fn snapshot_includes_macos_metric_slots() {
    let snapshot = collectors::CollectorSet::new().snapshot();
    assert!(snapshot
        .metrics
        .iter()
        .any(|metric| metric.name == "battery.percent"));
    assert!(snapshot
        .metrics
        .iter()
        .any(|metric| metric.name == "temperature.celsius"));
}
