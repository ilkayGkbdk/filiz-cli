pub mod processes;
pub mod system;

use std::time::SystemTime;

use crate::model::{CollectorResult, SystemSnapshot};
use processes::ProcessCollector;
use system::SystemCollector;

pub trait Collector {
    fn collect(&mut self) -> CollectorResult;
}

pub struct CollectorSet {
    system: SystemCollector,
    processes: ProcessCollector,
}

impl CollectorSet {
    pub fn new() -> Self {
        Self {
            system: SystemCollector::new(),
            processes: ProcessCollector::new(),
        }
    }

    pub fn snapshot(&mut self) -> SystemSnapshot {
        let captured_at = SystemTime::now();
        let mut system = self.system.collect();
        let processes = self.processes.collect();
        system.metrics.extend(processes.metrics);
        system.processes.extend(processes.processes);
        system.warnings.extend(processes.warnings);

        SystemSnapshot {
            captured_at,
            metrics: system.metrics,
            processes: system.processes,
            events: Vec::new(),
            warnings: system.warnings,
        }
    }
}

impl Default for CollectorSet {
    fn default() -> Self {
        Self::new()
    }
}
