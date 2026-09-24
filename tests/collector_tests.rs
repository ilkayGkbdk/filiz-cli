#[allow(dead_code)]
#[path = "../src/collectors/mod.rs"]
mod collectors;
#[allow(dead_code)]
#[path = "../src/model.rs"]
mod model;

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
    let result = collector.collect();
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
