use filiz::collectors::macos::{parse_battery, parse_cpu_usage};
use filiz::collectors::processes::ProcessCollector;
use filiz::collectors::system::{byte_rate, percentage, SystemCollector};

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
fn macos_cpu_parser_extracts_user_and_system_percentages() {
    assert_eq!(
        parse_cpu_usage("CPU usage: 12.50% user, 4.25% sys, 83.25% idle"),
        (Some(12.5), Some(4.25))
    );
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
fn system_sample_reports_memory_uptime_and_rates_after_second_sample() {
    let mut collector = SystemCollector::new();
    let first = collector.sample();
    assert!(first.memory.total.is_some());
    assert!(first.uptime.is_some());
    assert_eq!(first.cpu_usage, None, "first CPU sample has no baseline");
    assert!(first.interfaces.iter().all(|i| i.rx_rate.is_none()));
    let second = collector.sample();
    assert!(second.cpu_usage.is_some());
    assert!(second.processes.is_empty());
}

#[test]
fn process_sample_preserves_start_time_and_pid_order() {
    let mut collector = ProcessCollector::new();
    let processes = collector.sample();
    assert!(processes
        .windows(2)
        .all(|pair| pair[0].identity.pid < pair[1].identity.pid));
    let current_pid = std::process::id();
    let current = processes
        .iter()
        .find(|process| process.identity.pid == current_pid)
        .expect("current process is present");
    assert!(current.identity.start_time > 0);
    assert_eq!(current.cpu_percent, None);
    let second = collector.sample();
    let current = second
        .iter()
        .find(|process| process.identity.pid == current_pid)
        .unwrap();
    assert!(current.cpu_percent.is_some());
}
