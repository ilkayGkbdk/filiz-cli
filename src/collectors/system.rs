use std::collections::HashMap;
use std::time::{Duration, Instant};

use sysinfo::{Disks, Networks, System};

use crate::state::{is_system_mount, DiskStats, InterfaceStats, MemoryStats, SystemSample};

pub fn percentage(part: u64, total: u64) -> Option<f64> {
    (total > 0).then(|| part as f64 / total as f64 * 100.0)
}

pub fn byte_rate(previous: u64, current: u64, elapsed_seconds: f64) -> Option<f64> {
    if !elapsed_seconds.is_finite() || elapsed_seconds <= 0.0 {
        return None;
    }
    current
        .checked_sub(previous)
        .map(|delta| delta as f64 / elapsed_seconds)
}

pub struct SystemCollector {
    system: System,
    disks: Disks,
    networks: Networks,
    previous_networks: HashMap<String, NetworkState>,
    previous_at: Option<Instant>,
}

#[derive(Clone, Copy, Debug)]
struct NetworkState {
    received: u64,
    transmitted: u64,
    baseline_received: u64,
    baseline_transmitted: u64,
    peak_received: f64,
    peak_transmitted: f64,
}

impl SystemCollector {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            disks: Disks::new(),
            networks: Networks::new(),
            previous_networks: HashMap::new(),
            previous_at: None,
        }
    }

    pub fn sample(&mut self) -> SystemSample {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.disks.refresh_list();
        self.networks.refresh_list();
        let now = Instant::now();
        let elapsed = self
            .previous_at
            .map(|at| now.duration_since(at).as_secs_f64());
        let has_cpus = !self.system.cpus().is_empty();
        let cpu_usage = (has_cpus && self.previous_at.is_some())
            .then(|| f64::from(self.system.global_cpu_usage()));
        let interfaces = self.sample_interfaces(elapsed);
        self.previous_at = Some(now);
        let load = System::load_average();
        SystemSample {
            cpu_usage,
            cores: has_cpus.then(|| self.system.cpus().len()),
            load: Some([load.one, load.five, load.fifteen]),
            memory: self.sample_memory(),
            disks: self.sample_disks(),
            interfaces,
            processes: Vec::new(),
            uptime: Some(Duration::from_secs(System::uptime())),
        }
    }

    fn sample_memory(&self) -> MemoryStats {
        let total = self.system.total_memory();
        if total == 0 {
            return MemoryStats::default();
        }
        let used = self.system.used_memory();
        let available = match self.system.available_memory() {
            0 => total.saturating_sub(used),
            value => value,
        };
        MemoryStats {
            total: Some(total),
            used: Some(used),
            available: Some(available),
            free: Some(self.system.free_memory()),
            swap_used: Some(self.system.used_swap()),
            swap_total: Some(self.system.total_swap()),
        }
    }

    fn sample_disks(&self) -> Vec<DiskStats> {
        self.disks
            .list()
            .iter()
            .map(|disk| {
                let mount = disk.mount_point().to_string_lossy().into_owned();
                let total = disk.total_space();
                let free = disk.available_space();
                DiskStats {
                    is_system: is_system_mount(&mount),
                    mount,
                    total,
                    used: total.saturating_sub(free),
                    free,
                }
            })
            .collect()
    }

    fn sample_interfaces(&mut self, elapsed: Option<f64>) -> Vec<InterfaceStats> {
        let mut current = HashMap::new();
        let mut names: Vec<_> = self.networks.list().keys().cloned().collect();
        names.sort();
        let mut interfaces = Vec::with_capacity(names.len());
        for name in names {
            let network = &self.networks[&name];
            let received = network.total_received();
            let transmitted = network.total_transmitted();
            let previous = self.previous_networks.get(&name);
            let rx_rate = previous
                .zip(elapsed)
                .and_then(|(state, seconds)| byte_rate(state.received, received, seconds));
            let tx_rate = previous
                .zip(elapsed)
                .and_then(|(state, seconds)| byte_rate(state.transmitted, transmitted, seconds));
            let mut state = previous.copied().unwrap_or(NetworkState {
                received,
                transmitted,
                baseline_received: received,
                baseline_transmitted: transmitted,
                peak_received: 0.0,
                peak_transmitted: 0.0,
            });
            if received < state.received {
                state.baseline_received = received;
                state.peak_received = 0.0;
            }
            if transmitted < state.transmitted {
                state.baseline_transmitted = transmitted;
                state.peak_transmitted = 0.0;
            }
            if let Some(rate) = rx_rate {
                state.peak_received = state.peak_received.max(rate);
            }
            if let Some(rate) = tx_rate {
                state.peak_transmitted = state.peak_transmitted.max(rate);
            }
            interfaces.push(InterfaceStats {
                name: name.clone(),
                rx_rate,
                tx_rate,
                rx_total: received.saturating_sub(state.baseline_received),
                tx_total: transmitted.saturating_sub(state.baseline_transmitted),
                peak_rx: state.peak_received,
                peak_tx: state.peak_transmitted,
            });
            state.received = received;
            state.transmitted = transmitted;
            current.insert(name, state);
        }
        self.previous_networks = current;
        interfaces
    }
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}
