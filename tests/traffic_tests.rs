use std::sync::mpsc;
use std::time::{Duration, Instant};

use filiz::collectors::runtime::CollectorRuntime;
use filiz::collectors::traffic::{spawn, NettopParser, NettopRow, TrafficConfig, TrafficTracker};
use filiz::state::{CollectorUpdate, Source};

const HEADER: &str = "time,,interface,state,bytes_in,bytes_out,rx_dupe";

fn row(pid: u32, bytes_in: u64, bytes_out: u64) -> NettopRow {
    NettopRow {
        pid,
        bytes_in,
        bytes_out,
    }
}

#[test]
fn parser_emits_a_block_when_the_next_header_arrives() {
    let mut parser = NettopParser::default();
    assert_eq!(
        parser.push_line("11:00:00.1,Safari.321,,,1000,500,0"),
        None,
        "row before header"
    );
    assert_eq!(parser.push_line(HEADER), None);
    assert_eq!(parser.push_line("11:00:00.1,Safari.321,,,1000,500,0"), None);
    assert_eq!(parser.push_line("11:00:00.1,broken line"), None);
    assert_eq!(parser.push_line(HEADER), Some(vec![row(321, 1000, 500)]));
    assert_eq!(parser.push_line(HEADER), Some(vec![]));
}

#[test]
fn parser_takes_pid_after_last_dot() {
    let mut parser = NettopParser::default();
    parser.push_line(HEADER);
    parser.push_line("11:00:00.1,com.apple.WebKit.Networking.812,,,10,20,0");
    parser.push_line("11:00:00.1,Google Chrome H.81373,,,30,40,0");
    assert_eq!(
        parser.push_line(HEADER),
        Some(vec![row(812, 10, 20), row(81373, 30, 40)])
    );
}

#[test]
fn tracker_reports_rates_from_deltas_and_skips_idle_and_reset_counters() {
    let mut tracker = TrafficTracker::default();
    let start = Instant::now();
    assert!(tracker
        .update(
            &[row(1, 1000, 1000), row(2, 500, 500), row(3, 900, 900)],
            start
        )
        .is_empty());
    let rates = tracker.update(
        &[row(1, 3000, 1500), row(2, 500, 500), row(3, 100, 100)],
        start + Duration::from_secs(2),
    );
    assert_eq!(rates.len(), 1, "idle pid 2 and reset pid 3 are omitted");
    assert_eq!(rates[0].pid, 1);
    assert_eq!(rates[0].rate.rx, 1000.0);
    assert_eq!(rates[0].rate.tx, 250.0);
    let after_reset = tracker.update(&[row(3, 300, 100)], start + Duration::from_secs(4));
    assert_eq!(after_reset[0].rate.rx, 100.0);
}

fn fake(script: &str, max_restarts: u32) -> TrafficConfig {
    TrafficConfig {
        program: "/bin/sh".into(),
        args: vec!["-c".into(), script.into()],
        retry_delay: Duration::from_millis(10),
        max_restarts,
    }
}

#[test]
fn worker_streams_blocks_then_reports_exit() {
    let script = format!(
        "printf '{h}\\nx,Safari.321,,,1000,0,0\\n{h}\\n'; sleep 0.1; printf 'x,Safari.321,,,5000,0,0\\n{h}\\n'",
        h = HEADER
    );
    let (tx, rx) = mpsc::channel();
    let (_control, handle, _killer) = spawn(fake(&script, 0), tx);
    let wait = Duration::from_secs(2);
    assert_eq!(
        rx.recv_timeout(wait).unwrap(),
        CollectorUpdate::Traffic(vec![])
    );
    match rx.recv_timeout(wait).unwrap() {
        CollectorUpdate::Traffic(rows) => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].pid, 321);
            assert!(rows[0].rate.rx > 0.0);
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        rx.recv_timeout(wait).unwrap(),
        CollectorUpdate::Failed {
            source: Source::Traffic,
            message: "nettop exited".into()
        }
    );
    handle.join().unwrap();
}

#[test]
fn missing_program_is_reported_and_retried_up_to_the_limit() {
    let config = TrafficConfig {
        program: "/nonexistent/nettop".into(),
        args: Vec::new(),
        retry_delay: Duration::from_millis(10),
        max_restarts: 2,
    };
    let (tx, rx) = mpsc::channel();
    let (_control, handle, _killer) = spawn(config, tx);
    handle.join().unwrap();
    let failures: Vec<_> = rx.try_iter().collect();
    assert_eq!(failures.len(), 3);
    assert!(matches!(
        &failures[0],
        CollectorUpdate::Failed { source: Source::Traffic, message } if message.contains("unavailable")
    ));
}

#[test]
fn runtime_drop_kills_the_long_running_child() {
    let (tx, _rx) = mpsc::channel();
    let mut runtime = CollectorRuntime::with_workers(Vec::new(), tx.clone());
    let (control, handle, killer) = spawn(
        fake("printf 'time,,bytes_in,bytes_out\\n'; exec sleep 30", 3),
        tx,
    );
    runtime.adopt(control, handle);
    runtime.add_child_killer(killer);
    std::thread::sleep(Duration::from_millis(200));
    let started = Instant::now();
    drop(runtime);
    assert!(started.elapsed() < Duration::from_secs(2));
}
