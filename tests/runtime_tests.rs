use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use filiz::app::App;
use filiz::collectors::runtime::{CollectorRuntime, WorkerSpec};
use filiz::state::{CollectorUpdate, PlatformSample, Source, SystemSample};

fn counting_worker(interval: Duration, delay: Duration, counter: Arc<AtomicUsize>) -> WorkerSpec {
    WorkerSpec {
        source: Source::Platform,
        interval,
        job: Box::new(move || {
            std::thread::sleep(delay);
            counter.fetch_add(1, Ordering::SeqCst);
            CollectorUpdate::Platform(PlatformSample::default())
        }),
    }
}

#[test]
fn worker_runs_immediately_and_then_on_interval() {
    let (tx, rx) = mpsc::channel();
    let counter = Arc::new(AtomicUsize::new(0));
    let runtime = CollectorRuntime::with_workers(
        vec![counting_worker(
            Duration::from_millis(50),
            Duration::ZERO,
            counter.clone(),
        )],
        tx,
    );
    let first = rx.recv_timeout(Duration::from_millis(500)).unwrap();
    assert!(matches!(first, CollectorUpdate::Platform(_)));
    std::thread::sleep(Duration::from_millis(180));
    drop(runtime);
    assert!(counter.load(Ordering::SeqCst) >= 3);
}

#[test]
fn refresh_triggers_collection_before_interval() {
    let (tx, rx) = mpsc::channel();
    let counter = Arc::new(AtomicUsize::new(0));
    let runtime = CollectorRuntime::with_workers(
        vec![counting_worker(
            Duration::from_secs(60),
            Duration::ZERO,
            counter.clone(),
        )],
        tx,
    );
    rx.recv_timeout(Duration::from_millis(500)).unwrap();
    runtime.refresh();
    rx.recv_timeout(Duration::from_millis(500)).unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[test]
fn pump_never_blocks_on_slow_worker() {
    let (tx, rx) = mpsc::channel();
    let _runtime = CollectorRuntime::with_workers(
        vec![counting_worker(
            Duration::from_secs(60),
            Duration::from_secs(1),
            Arc::new(AtomicUsize::new(0)),
        )],
        tx,
    );
    let mut app = App::new(Duration::from_secs(2));
    let started = Instant::now();
    assert!(!app.pump(&rx));
    assert!(started.elapsed() < Duration::from_millis(20));
}

#[test]
fn pump_applies_every_queued_update() {
    let (tx, rx) = mpsc::channel();
    tx.send(CollectorUpdate::System(SystemSample {
        cpu_usage: Some(10.0),
        ..Default::default()
    }))
    .unwrap();
    tx.send(CollectorUpdate::System(SystemSample {
        cpu_usage: Some(20.0),
        ..Default::default()
    }))
    .unwrap();
    let mut app = App::new(Duration::from_secs(2));
    assert!(app.pump(&rx));
    assert_eq!(app.state.cpu.usage, Some(20.0));
}

#[test]
fn runtime_drop_joins_workers() {
    let (tx, rx) = mpsc::channel();
    let runtime = CollectorRuntime::with_workers(
        vec![counting_worker(
            Duration::from_secs(60),
            Duration::ZERO,
            Arc::new(AtomicUsize::new(0)),
        )],
        tx,
    );
    rx.recv_timeout(Duration::from_millis(500)).unwrap();
    let started = Instant::now();
    drop(runtime);
    assert!(started.elapsed() < Duration::from_millis(500));
    assert!(matches!(
        rx.recv_timeout(Duration::from_millis(100)),
        Err(mpsc::RecvTimeoutError::Disconnected)
    ));
}

#[test]
fn queued_refreshes_do_not_delay_shutdown() {
    let (tx, rx) = mpsc::channel();
    let runtime = CollectorRuntime::with_workers(
        vec![counting_worker(
            Duration::from_secs(60),
            Duration::from_millis(100),
            Arc::new(AtomicUsize::new(0)),
        )],
        tx,
    );
    rx.recv_timeout(Duration::from_millis(500)).unwrap();
    for _ in 0..50 {
        runtime.refresh();
    }
    let started = Instant::now();
    drop(runtime);
    assert!(started.elapsed() < Duration::from_millis(500));
}

#[test]
fn panicking_worker_reports_stopped_source() {
    let (tx, rx) = mpsc::channel();
    let _runtime = CollectorRuntime::with_workers(
        vec![WorkerSpec {
            source: Source::Traffic,
            interval: Duration::from_secs(60),
            job: Box::new(|| panic!("collector bug")),
        }],
        tx,
    );
    let update = rx.recv_timeout(Duration::from_millis(500)).unwrap();
    assert_eq!(update, CollectorUpdate::Stopped(Source::Traffic));
}
