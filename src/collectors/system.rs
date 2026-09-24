use std::collections::HashMap;
use std::time::Instant;

use sysinfo::{Disks, Networks, System};

use crate::model::{
    CollectorData, CollectorResult, CollectorWarning, NetworkSummary, ResourceMetric,
};

use super::Collector;

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
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for SystemCollector {
    fn collect(&mut self) -> CollectorResult {
        let mut result = CollectorData::default();
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.disks.refresh_list();
        self.networks.refresh_list();

        let now = Instant::now();
        let elapsed = self
            .previous_at
            .map(|at| now.duration_since(at).as_secs_f64());
        let cpu_value = (!self.system.cpus().is_empty() && self.previous_at.is_some())
            .then(|| f64::from(self.system.global_cpu_usage()));
        metric(&mut result, "cpu.usage", cpu_value, "%");
        metric(
            &mut result,
            "cpu.idle",
            cpu_value.map(|usage| (100.0 - usage).max(0.0)),
            "%",
        );
        if self.system.cpus().is_empty() {
            warning(&mut result, "CPU data unavailable");
        }
        metric(
            &mut result,
            "cpu.cores",
            (!self.system.cpus().is_empty()).then(|| self.system.cpus().len() as f64),
            "cores",
        );

        let total = self.system.total_memory();
        let used = self.system.used_memory();
        metric(
            &mut result,
            "memory.total",
            (total > 0).then_some(total as f64),
            "B",
        );
        metric(
            &mut result,
            "memory.used",
            (total > 0).then_some(used as f64),
            "B",
        );
        metric(
            &mut result,
            "memory.available",
            (total > 0).then_some(self.system.available_memory() as f64),
            "B",
        );
        metric(
            &mut result,
            "memory.free",
            (total > 0).then_some(self.system.free_memory() as f64),
            "B",
        );
        metric(&mut result, "memory.cached", None, "B");
        metric(&mut result, "memory.usage", percentage(used, total), "%");
        if total == 0 {
            warning(&mut result, "memory capacity unavailable");
        }
        metric(&mut result, "uptime", Some(System::uptime() as f64), "s");

        if self.disks.list().is_empty() {
            metric(&mut result, "disk.usage", None, "%");
            warning(&mut result, "no disks available");
        }
        for disk in self.disks.list() {
            let mount = disk.mount_point().to_string_lossy();
            let total = disk.total_space();
            let used = total.saturating_sub(disk.available_space());
            metric(
                &mut result,
                &format!("disk.{mount}.total"),
                (total > 0).then_some(total as f64),
                "B",
            );
            metric(
                &mut result,
                &format!("disk.{mount}.used"),
                (total > 0).then_some(used as f64),
                "B",
            );
            metric(
                &mut result,
                &format!("disk.{mount}.free"),
                (total > 0).then_some(disk.available_space() as f64),
                "B",
            );
            metric(
                &mut result,
                &format!("disk.{mount}.usage"),
                percentage(used, total),
                "%",
            );
            if total == 0 {
                warning(&mut result, &format!("disk capacity unavailable: {mount}"));
            }
        }

        if self.networks.list().is_empty() {
            metric(&mut result, "network.received", None, "B/s");
            metric(&mut result, "network.transmitted", None, "B/s");
            warning(&mut result, "no network interfaces available");
        }
        let mut current_networks = HashMap::new();
        let mut names: Vec<_> = self.networks.list().keys().collect();
        names.sort();
        for name in names {
            let network = &self.networks[name];
            let received = network.total_received();
            let transmitted = network.total_transmitted();
            let previous = self.previous_networks.get(name);
            let rx_rate = previous
                .zip(elapsed)
                .and_then(|(state, seconds)| byte_rate(state.received, received, seconds));
            let tx_rate = previous
                .zip(elapsed)
                .and_then(|(state, seconds)| byte_rate(state.transmitted, transmitted, seconds));
            metric(
                &mut result,
                &format!("network.{name}.received"),
                rx_rate,
                "B/s",
            );
            metric(
                &mut result,
                &format!("network.{name}.transmitted"),
                tx_rate,
                "B/s",
            );
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
            let download_rate = rx_rate;
            let upload_rate = tx_rate;
            if let Some(rate) = download_rate {
                state.peak_received = state.peak_received.max(rate);
            }
            if let Some(rate) = upload_rate {
                state.peak_transmitted = state.peak_transmitted.max(rate);
            }
            result.network_summaries.push(NetworkSummary {
                interface: name.clone(),
                download_rate,
                upload_rate,
                download_total: received.saturating_sub(state.baseline_received),
                upload_total: transmitted.saturating_sub(state.baseline_transmitted),
                peak_download: state.peak_received,
                peak_upload: state.peak_transmitted,
                connections: Vec::new(),
            });
            state.received = received;
            state.transmitted = transmitted;
            current_networks.insert(name.clone(), state);
        }
        self.previous_networks = current_networks;
        self.previous_at = Some(now);
        Ok(result)
    }
}

fn metric(result: &mut CollectorData, name: &str, value: Option<f64>, unit: &str) {
    result.metrics.push(ResourceMetric {
        name: name.to_owned(),
        value,
        unit: unit.to_owned(),
    });
}

fn warning(result: &mut CollectorData, message: &str) {
    result.warnings.push(CollectorWarning {
        collector: "system".to_owned(),
        message: message.to_owned(),
    });
}
