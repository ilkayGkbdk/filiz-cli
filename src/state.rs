use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use crate::model::{ProcessInfo, TrafficRate};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    System,
    Platform,
    Traffic,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CpuStats {
    pub usage: Option<f64>,
    pub user: Option<f64>,
    pub system: Option<f64>,
    pub cores: Option<usize>,
    pub load: Option<[f64; 3]>,
    pub temperature: Option<f64>,
}

impl CpuStats {
    pub fn idle(&self) -> Option<f64> {
        self.usage.map(|usage| (100.0 - usage).max(0.0))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemoryStats {
    pub total: Option<u64>,
    pub used: Option<u64>,
    pub available: Option<u64>,
    pub free: Option<u64>,
    pub swap_used: Option<u64>,
    pub swap_total: Option<u64>,
}

impl MemoryStats {
    pub fn usage_percent(&self) -> Option<f64> {
        match (self.used, self.total) {
            (Some(used), Some(total)) if total > 0 => Some(used as f64 / total as f64 * 100.0),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiskStats {
    pub mount: String,
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub is_system: bool,
}

impl DiskStats {
    pub fn usage_percent(&self) -> Option<f64> {
        (self.total > 0).then(|| self.used as f64 / self.total as f64 * 100.0)
    }
}

pub fn is_system_mount(mount: &str) -> bool {
    mount.starts_with("/System/Volumes/") || mount.starts_with("/private/var/vm")
}

#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceStats {
    pub name: String,
    pub rx_rate: Option<f64>,
    pub tx_rate: Option<f64>,
    pub rx_total: u64,
    pub tx_total: u64,
    pub peak_rx: f64,
    pub peak_tx: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Battery {
    pub percent: f64,
    pub charging: Option<bool>,
    pub power_source: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessTraffic {
    pub pid: u32,
    pub rate: TrafficRate,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemSample {
    pub cpu_usage: Option<f64>,
    pub cores: Option<usize>,
    pub load: Option<[f64; 3]>,
    pub memory: MemoryStats,
    pub disks: Vec<DiskStats>,
    pub interfaces: Vec<InterfaceStats>,
    pub processes: Vec<ProcessInfo>,
    pub uptime: Option<Duration>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlatformSample {
    pub cpu_user: Option<f64>,
    pub cpu_system: Option<f64>,
    pub battery: Option<Battery>,
    pub temperature: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CollectorUpdate {
    System(SystemSample),
    Platform(PlatformSample),
    Traffic(Vec<ProcessTraffic>),
    Failed { source: Source, message: String },
    Stopped(Source),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemState {
    pub cpu: CpuStats,
    pub memory: MemoryStats,
    pub disks: Vec<DiskStats>,
    pub interfaces: Vec<InterfaceStats>,
    pub processes: Vec<ProcessInfo>,
    pub battery: Option<Battery>,
    pub uptime: Option<Duration>,
    issues: BTreeMap<Source, String>,
    traffic: HashMap<u32, TrafficRate>,
}

impl SystemState {
    pub fn apply(&mut self, update: CollectorUpdate) {
        match update {
            CollectorUpdate::System(sample) => {
                self.cpu.usage = sample.cpu_usage;
                self.cpu.cores = sample.cores;
                self.cpu.load = sample.load;
                self.memory = sample.memory;
                self.disks = sample.disks;
                self.interfaces = sample.interfaces;
                self.processes = sample.processes;
                self.uptime = sample.uptime;
                self.attach_traffic();
                self.issues.remove(&Source::System);
            }
            CollectorUpdate::Platform(sample) => {
                self.cpu.user = sample.cpu_user;
                self.cpu.system = sample.cpu_system;
                self.cpu.temperature = sample.temperature;
                self.battery = sample.battery;
                self.issues.remove(&Source::Platform);
            }
            CollectorUpdate::Traffic(rows) => {
                self.traffic = rows.into_iter().map(|row| (row.pid, row.rate)).collect();
                self.attach_traffic();
                self.issues.remove(&Source::Traffic);
            }
            CollectorUpdate::Failed { source, message } => {
                self.clear(source);
                self.issues.insert(source, message);
            }
            CollectorUpdate::Stopped(source) => {
                self.issues.insert(source, "data stream stopped".into());
            }
        }
    }

    pub fn has_failures(&self) -> bool {
        !self.issues.is_empty()
    }

    pub fn issue(&self, source: Source) -> Option<&str> {
        self.issues.get(&source).map(String::as_str)
    }

    pub fn issues(&self) -> impl Iterator<Item = (Source, &str)> {
        self.issues
            .iter()
            .map(|(source, message)| (*source, message.as_str()))
    }

    pub fn primary_disk(&self) -> Option<&DiskStats> {
        self.disks
            .iter()
            .find(|disk| disk.mount == "/")
            .or_else(|| self.disks.iter().find(|disk| !disk.is_system))
    }

    pub fn visible_disks(&self) -> Vec<&DiskStats> {
        self.disks.iter().filter(|disk| !disk.is_system).collect()
    }

    fn attach_traffic(&mut self) {
        for process in &mut self.processes {
            process.traffic = self.traffic.get(&process.identity.pid).copied();
        }
    }

    fn clear(&mut self, source: Source) {
        match source {
            Source::System => {
                self.cpu.usage = None;
                self.cpu.cores = None;
                self.cpu.load = None;
                self.memory = MemoryStats::default();
                self.disks.clear();
                self.interfaces.clear();
                self.processes.clear();
                self.uptime = None;
            }
            Source::Platform => {
                self.cpu.user = None;
                self.cpu.system = None;
                self.cpu.temperature = None;
                self.battery = None;
            }
            Source::Traffic => {
                self.traffic.clear();
                self.attach_traffic();
            }
        }
    }
}
