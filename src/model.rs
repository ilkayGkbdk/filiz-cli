use std::time::SystemTime;

/// A single measured resource value. `None` represents an unavailable metric.
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceMetric {
    pub name: String,
    pub value: Option<f64>,
    pub unit: String,
}

/// Stable process identity for distinguishing PID reuse across process lifetimes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub start_time: u64,
}

/// Process data captured during one collection cycle.
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessInfo {
    pub identity: ProcessIdentity,
    pub name: String,
    pub command: String,
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
    pub user: Option<String>,
    pub status: Option<String>,
}

/// An event suitable for display in the recent-events panel.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemEvent {
    pub timestamp: SystemTime,
    pub message: String,
}

/// A non-fatal issue reported by an individual collector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectorWarning {
    pub collector: String,
    pub message: String,
}

/// Data returned by one collector. Missing measurements stay local to the collector.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollectorResult {
    pub metrics: Vec<ResourceMetric>,
    pub processes: Vec<ProcessInfo>,
    pub warnings: Vec<CollectorWarning>,
}

/// The data gathered during one system refresh.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemSnapshot {
    pub captured_at: SystemTime,
    pub metrics: Vec<ResourceMetric>,
    pub processes: Vec<ProcessInfo>,
    pub events: Vec<SystemEvent>,
    pub warnings: Vec<CollectorWarning>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortMode {
    Cpu,
    Memory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppMode {
    Dashboard,
    ProcessDetail,
    Filtering,
    ConfirmingAction,
}

/// Return processes whose name or command contains `query`, ignoring case.
/// An empty or whitespace-only query returns every process.
pub fn filter_processes(processes: &[ProcessInfo], query: &str) -> Vec<ProcessInfo> {
    let query = query.trim();
    if query.is_empty() {
        return processes.to_vec();
    }

    let query = query.to_lowercase();
    processes
        .iter()
        .filter(|process| {
            process.name.to_lowercase().contains(&query)
                || process.command.to_lowercase().contains(&query)
        })
        .cloned()
        .collect()
}

/// Sort processes by the selected resource, highest first, then by PID.
/// Missing measurements sort after available values.
pub fn sort_processes(processes: &mut [ProcessInfo], mode: SortMode) {
    processes.sort_by(|left, right| {
        let resource_order = match mode {
            SortMode::Cpu => compare_optional_f32_desc(left.cpu_percent, right.cpu_percent),
            SortMode::Memory => compare_optional_u64_desc(left.memory_bytes, right.memory_bytes),
        };

        resource_order.then_with(|| left.identity.pid.cmp(&right.identity.pid))
    });
}

fn compare_optional_f32_desc(left: Option<f32>, right: Option<f32>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.total_cmp(&left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn compare_optional_u64_desc(left: Option<u64>, right: Option<u64>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
