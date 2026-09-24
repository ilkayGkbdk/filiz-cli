use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind, Users};

use crate::model::{CollectorResult, CollectorWarning, ProcessIdentity, ProcessInfo};

use super::Collector;

pub struct ProcessCollector {
    system: System,
    users: Users,
}

impl ProcessCollector {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            users: Users::new_with_refreshed_list(),
        }
    }
}

impl Default for ProcessCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for ProcessCollector {
    fn collect(&mut self) -> CollectorResult {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::new()
                .with_cpu()
                .with_memory()
                .with_cmd(UpdateKind::OnlyIfNotSet)
                .with_user(UpdateKind::OnlyIfNotSet),
        );

        let mut result = CollectorResult::default();
        for (pid, process) in self.system.processes() {
            let command = process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");
            let user = process.user_id().and_then(|id| {
                self.users
                    .list()
                    .iter()
                    .find(|user| user.id() == id)
                    .map(|user| user.name().to_owned())
            });
            result.processes.push(ProcessInfo {
                identity: ProcessIdentity {
                    pid: pid.as_u32(),
                    start_time: process.start_time(),
                },
                name: process.name().to_string_lossy().into_owned(),
                command,
                cpu_percent: Some(process.cpu_usage()),
                memory_bytes: Some(process.memory()),
                user,
                status: Some(process.status().to_string()),
            });
        }
        result.processes.sort_by_key(|process| process.identity.pid);
        if result.processes.is_empty() {
            result.warnings.push(CollectorWarning {
                collector: "processes".to_owned(),
                message: "no processes available".to_owned(),
            });
        }
        result
    }
}
