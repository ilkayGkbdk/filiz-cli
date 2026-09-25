use std::time::Duration;

use filiz::history::{History, SeriesKey};
use filiz::model::{ProcessIdentity, ProcessInfo, TrafficRate};
use filiz::state::{
    is_system_mount, Battery, CollectorUpdate, DiskStats, InterfaceStats, MemoryStats,
    PlatformSample, ProcessTraffic, Source, SystemSample, SystemState,
};

fn process(pid: u32) -> ProcessInfo {
    ProcessInfo {
        identity: ProcessIdentity { pid, start_time: 1 },
        name: format!("p{pid}"),
        command: String::new(),
        cpu_percent: Some(1.0),
        memory_bytes: Some(1),
        user: None,
        status: None,
        traffic: None,
    }
}

fn iface(name: &str, rx: Option<f64>, tx: Option<f64>) -> InterfaceStats {
    InterfaceStats {
        name: name.into(),
        rx_rate: rx,
        tx_rate: tx,
        rx_total: 0,
        tx_total: 0,
        peak_rx: 0.0,
        peak_tx: 0.0,
    }
}

fn disk(mount: &str, used: u64) -> DiskStats {
    DiskStats {
        mount: mount.into(),
        total: 100,
        used,
        free: 100 - used,
        is_system: is_system_mount(mount),
    }
}

fn system_sample() -> SystemSample {
    SystemSample {
        cpu_usage: Some(40.0),
        cores: Some(8),
        load: Some([1.0, 2.0, 3.0]),
        memory: MemoryStats {
            total: Some(100),
            used: Some(25),
            ..Default::default()
        },
        disks: vec![disk("/System/Volumes/VM", 10), disk("/", 70)],
        interfaces: vec![iface("en0", Some(100.0), Some(50.0))],
        processes: vec![process(1), process(2)],
        uptime: Some(Duration::from_secs(60)),
    }
}

fn platform_sample() -> PlatformSample {
    PlatformSample {
        cpu_user: Some(10.0),
        cpu_system: Some(5.0),
        battery: Some(Battery {
            percent: 80.0,
            charging: Some(true),
            power_source: Some("AC Power".into()),
        }),
        temperature: None,
    }
}

#[test]
fn system_update_leaves_platform_fields_untouched() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::Platform(platform_sample()));
    state.apply(CollectorUpdate::System(system_sample()));
    assert_eq!(state.cpu.usage, Some(40.0));
    assert_eq!(state.cpu.user, Some(10.0));
    assert_eq!(state.battery.as_ref().unwrap().percent, 80.0);
    assert_eq!(state.cpu.idle(), Some(60.0));
    assert_eq!(state.memory.usage_percent(), Some(25.0));
}

#[test]
fn missing_sensor_is_not_a_failure() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::Platform(platform_sample()));
    assert_eq!(state.cpu.temperature, None);
    assert!(!state.has_failures());
}

#[test]
fn failure_clears_only_that_source_and_recovers() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::System(system_sample()));
    state.apply(CollectorUpdate::Platform(platform_sample()));
    state.apply(CollectorUpdate::Failed {
        source: Source::Platform,
        message: "top failed".into(),
    });
    assert_eq!(state.cpu.user, None);
    assert!(state.battery.is_none());
    assert_eq!(state.cpu.usage, Some(40.0));
    assert_eq!(state.issue(Source::Platform), Some("top failed"));
    assert!(state.has_failures());

    state.apply(CollectorUpdate::Platform(platform_sample()));
    assert!(!state.has_failures());
}

#[test]
fn stopped_worker_is_reported_as_issue() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::Stopped(Source::System));
    assert_eq!(state.issue(Source::System), Some("data stream stopped"));
}

#[test]
fn traffic_is_attached_to_matching_processes_across_system_updates() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::System(system_sample()));
    state.apply(CollectorUpdate::Traffic(vec![ProcessTraffic {
        pid: 2,
        rate: TrafficRate { rx: 10.0, tx: 5.0 },
    }]));
    let find = |state: &SystemState, pid| {
        state
            .processes
            .iter()
            .find(|p| p.identity.pid == pid)
            .unwrap()
            .traffic
    };
    assert_eq!(find(&state, 2), Some(TrafficRate { rx: 10.0, tx: 5.0 }));
    assert_eq!(find(&state, 1), None);

    state.apply(CollectorUpdate::System(system_sample()));
    assert_eq!(find(&state, 2), Some(TrafficRate { rx: 10.0, tx: 5.0 }));
}

#[test]
fn system_volumes_are_hidden_and_root_is_primary() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::System(system_sample()));
    assert_eq!(state.primary_disk().unwrap().mount, "/");
    let visible: Vec<_> = state
        .visible_disks()
        .iter()
        .map(|d| d.mount.clone())
        .collect();
    assert_eq!(visible, vec!["/".to_string()]);
    assert!(is_system_mount("/private/var/vm"));
    assert!(!is_system_mount("/Volumes/USB"));
}

#[test]
fn history_skips_missing_values_and_respects_capacity() {
    let mut history = History::new(3);
    for value in [Some(1.0), None, Some(2.0), Some(3.0), Some(4.0)] {
        history.push(SeriesKey::Cpu, value);
    }
    assert_eq!(history.series(&SeriesKey::Cpu), vec![2, 3, 4]);
}

#[test]
fn history_records_rx_and_tx_separately_per_interface() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::System(system_sample()));
    let mut history = History::new(60);
    history.record(&state);
    assert_eq!(history.series(&SeriesKey::NetRx("en0".into())), vec![100]);
    assert_eq!(history.series(&SeriesKey::NetTx("en0".into())), vec![50]);
    assert_eq!(history.series(&SeriesKey::Cpu), vec![40]);
    assert_eq!(history.series(&SeriesKey::Disk), vec![70]);
}

#[test]
fn history_drops_series_for_vanished_interfaces() {
    let mut state = SystemState::default();
    let mut sample = system_sample();
    sample.interfaces.push(iface("utun3", Some(1.0), Some(1.0)));
    state.apply(CollectorUpdate::System(sample));
    let mut history = History::new(60);
    history.record(&state);
    assert_eq!(history.len(&SeriesKey::NetRx("utun3".into())), 1);

    state.apply(CollectorUpdate::System(system_sample()));
    history.record(&state);
    assert_eq!(history.len(&SeriesKey::NetRx("utun3".into())), 0);
    assert_eq!(history.len(&SeriesKey::NetRx("en0".into())), 2);
}
