#[allow(dead_code)]
#[path = "../src/model.rs"]
mod model;

use model::{filter_processes, sort_processes, ProcessInfo, SortMode};

fn process(
    pid: u32,
    name: &str,
    command: &str,
    cpu_percent: f32,
    memory_bytes: u64,
) -> ProcessInfo {
    ProcessInfo {
        pid,
        name: name.to_owned(),
        command: command.to_owned(),
        cpu_percent: Some(cpu_percent),
        memory_bytes: Some(memory_bytes),
        user: None,
        status: None,
    }
}

#[test]
fn model_tests_filter_matches_name_and_command_case_insensitively() {
    let processes = vec![
        process(1, "Terminal", "/Applications/Terminal.app", 1.0, 100),
        process(2, "worker", "/usr/local/bin/BUILD-WORKER", 2.0, 200),
        process(3, "Finder", "/System/Library/Finder", 3.0, 300),
    ];

    let results = filter_processes(&processes, "tErMiNaL");
    assert_eq!(results.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![1]);

    let results = filter_processes(&processes, "build-worker");
    assert_eq!(results.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![2]);
}

#[test]
fn model_tests_sort_orders_cpu_descending_with_pid_tie_breaker() {
    let mut processes = vec![
        process(8, "eight", "eight", 10.0, 100),
        process(3, "three", "three", 50.0, 100),
        process(2, "two", "two", 50.0, 100),
    ];

    sort_processes(&mut processes, SortMode::Cpu);
    assert_eq!(
        processes.iter().map(|p| p.pid).collect::<Vec<_>>(),
        vec![2, 3, 8]
    );
}

#[test]
fn model_tests_sort_orders_memory_descending_with_pid_tie_breaker() {
    let mut processes = vec![
        process(8, "eight", "eight", 100.0, 200),
        process(3, "three", "three", 100.0, 900),
        process(2, "two", "two", 100.0, 900),
    ];

    sort_processes(&mut processes, SortMode::Memory);
    assert_eq!(
        processes.iter().map(|p| p.pid).collect::<Vec<_>>(),
        vec![2, 3, 8]
    );
}
