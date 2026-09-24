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
        snapshot_from_collectors(&mut self.system, &mut self.processes)
    }
}

fn snapshot_from_collectors<'a>(
    system: &'a mut dyn Collector,
    processes: &'a mut dyn Collector,
) -> SystemSnapshot {
    let captured_at = SystemTime::now();
    let mut metrics = Vec::new();
    let mut process_list = Vec::new();
    let mut warnings = Vec::new();
    for collector in [system, processes] {
        match collector.collect() {
            Ok(section) => {
                metrics.extend(section.metrics);
                process_list.extend(section.processes);
                warnings.extend(section.warnings);
            }
            Err(warning) => warnings.push(warning),
        }
    }

    SystemSnapshot {
        captured_at,
        metrics,
        processes: process_list,
        events: Vec::new(),
        warnings,
    }
}

impl Default for CollectorSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CollectorWarning, ResourceMetric};

    struct FailingCollector;

    impl Collector for FailingCollector {
        fn collect(&mut self) -> CollectorResult {
            Err(CollectorWarning {
                collector: "failed".to_owned(),
                message: "sample unavailable".to_owned(),
            })
        }
    }

    struct WorkingCollector;

    impl Collector for WorkingCollector {
        fn collect(&mut self) -> CollectorResult {
            Ok(crate::model::CollectorData {
                metrics: vec![ResourceMetric {
                    name: "working".to_owned(),
                    value: Some(1.0),
                    unit: "count".to_owned(),
                }],
                ..Default::default()
            })
        }
    }

    #[test]
    fn failing_collector_does_not_discard_a_successful_section() {
        let snapshot = snapshot_from_collectors(&mut FailingCollector, &mut WorkingCollector);
        assert_eq!(snapshot.metrics[0].name, "working");
        assert_eq!(snapshot.warnings.len(), 1);
        assert_eq!(snapshot.warnings[0].collector, "failed");
    }
}
