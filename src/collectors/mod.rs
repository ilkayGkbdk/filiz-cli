pub mod macos;
pub mod processes;
pub mod system;

use crate::state::CollectorUpdate;
use macos::MacOsCollector;
use processes::ProcessCollector;
use system::SystemCollector;

pub struct CollectorSet {
    system: SystemCollector,
    processes: ProcessCollector,
    macos: MacOsCollector,
}

impl CollectorSet {
    pub fn new() -> Self {
        Self {
            system: SystemCollector::new(),
            processes: ProcessCollector::new(),
            macos: MacOsCollector::new(),
        }
    }

    pub fn system_update(&mut self) -> CollectorUpdate {
        let mut sample = self.system.sample();
        sample.processes = self.processes.sample();
        CollectorUpdate::System(sample)
    }

    pub fn platform_update(&mut self) -> CollectorUpdate {
        CollectorUpdate::Platform(self.macos.sample())
    }

    pub fn collect_all(&mut self) -> Vec<CollectorUpdate> {
        vec![self.system_update(), self.platform_update()]
    }
}

impl Default for CollectorSet {
    fn default() -> Self {
        Self::new()
    }
}
