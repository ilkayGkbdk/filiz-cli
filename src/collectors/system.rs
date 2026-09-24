use std::collections::HashMap;
use std::process::Command;
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
        let available = self.system.available_memory();
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
            (total > 0).then_some(if available > 0 {
                available
            } else {
                total.saturating_sub(used)
            } as f64),
            "B",
        );
        metric(
            &mut result,
            "memory.free",
            (total > 0).then_some(self.system.free_memory() as f64),
            "B",
        );
        metric(&mut result, "memory.cached", None, "B");
        metric(
            &mut result,
            "memory.swap.total",
            Some(self.system.total_swap() as f64),
            "B",
        );
        metric(
            &mut result,
            "memory.swap.used",
            Some(self.system.used_swap() as f64),
            "B",
        );
        metric(
            &mut result,
            "memory.swap.free",
            Some(self.system.free_swap() as f64),
            "B",
        );
        metric(&mut result, "memory.usage", percentage(used, total), "%");
        if total == 0 {
            warning(&mut result, "memory capacity unavailable");
        }
        metric(&mut result, "uptime", Some(System::uptime() as f64), "s");
        let load = System::load_average();
        metric(&mut result, "load.1", Some(load.one), "load");
        metric(&mut result, "load.5", Some(load.five), "load");
        metric(&mut result, "load.15", Some(load.fifteen), "load");

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
            metric(&mut result, &format!("disk.{mount}.read"), None, "B/s");
            metric(&mut result, &format!("disk.{mount}.write"), None, "B/s");
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
        let connections = collect_connections();
        let mut names: Vec<_> = self.networks.list().keys().collect();
        names.sort();
        for (index, name) in names.into_iter().enumerate() {
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
                connections: if index == 0 {
                    connections.clone()
                } else {
                    Vec::new()
                },
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

fn collect_connections() -> Vec<crate::model::ConnectionSummary> {
    let nettop = collect_nettop_connections();
    if !nettop.is_empty() {
        return nettop;
    }
    let Ok(output) = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:ESTABLISHED"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            let process = fields.first()?.to_string();
            let remote = fields
                .iter()
                .find(|field| field.contains("->"))?
                .split("->")
                .nth(1)?
                .trim_end_matches("(ESTABLISHED)")
                .to_string();
            Some(crate::model::ConnectionSummary {
                process,
                remote,
                direction: "TCP".to_owned(),
                bytes_per_second: None,
            })
        })
        .take(24)
        .collect()
}

pub fn parse_nettop_csv(output: &str) -> Vec<crate::model::ConnectionSummary> {
    let mut lines = output.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let columns = header.split(',').map(str::trim).collect::<Vec<_>>();
    let index = |name: &str| columns.iter().position(|column| *column == name);
    let process_index = index("process").or_else(|| index("proc"));
    let remote_index = index("remote").or_else(|| index("remote_addr"));
    let incoming_index = index("bytes_in").or_else(|| index("rx_bytes"));
    let outgoing_index = index("bytes_out").or_else(|| index("tx_bytes"));
    let Some(process_index) = process_index else {
        return Vec::new();
    };
    lines
        .filter_map(|line| {
            let fields = line.split(',').map(str::trim).collect::<Vec<_>>();
            let process = fields.get(process_index)?.trim_matches('"').to_owned();
            if process.is_empty() {
                return None;
            }
            let remote = remote_index
                .and_then(|index| fields.get(index))
                .map(|value| value.trim_matches('"').to_owned())
                .unwrap_or_else(|| "N/A".to_owned());
            let incoming = incoming_index
                .and_then(|index| fields.get(index))
                .and_then(|value| value.trim_matches('"').parse::<f64>().ok());
            let outgoing = outgoing_index
                .and_then(|index| fields.get(index))
                .and_then(|value| value.trim_matches('"').parse::<f64>().ok());
            Some(crate::model::ConnectionSummary {
                process,
                remote,
                direction: "NET".to_owned(),
                bytes_per_second: match (incoming, outgoing) {
                    (Some(incoming), Some(outgoing)) => Some(incoming + outgoing),
                    (Some(value), None) | (None, Some(value)) => Some(value),
                    _ => None,
                },
            })
        })
        .take(24)
        .collect()
}

fn collect_nettop_connections() -> Vec<crate::model::ConnectionSummary> {
    let Ok(output) = Command::new("nettop")
        .args([
            "-P",
            "-L",
            "1",
            "-x",
            "-J",
            "process,remote,bytes_in,bytes_out",
        ])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_nettop_csv(&String::from_utf8_lossy(&output.stdout))
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
