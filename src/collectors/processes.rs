use std::collections::HashSet;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind, Users};

use crate::model::{ProcessIdentity, ProcessInfo};

pub struct ProcessCollector {
    system: System,
    users: Users,
    previous_processes: HashSet<ProcessIdentity>,
}

impl ProcessCollector {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            users: Users::new_with_refreshed_list(),
            previous_processes: HashSet::new(),
        }
    }

    pub fn sample(&mut self) -> Vec<ProcessInfo> {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::new()
                .with_cpu()
                .with_memory()
                .with_cmd(UpdateKind::OnlyIfNotSet)
                .with_user(UpdateKind::OnlyIfNotSet),
        );
        let mut processes = Vec::new();
        let mut current = HashSet::new();
        for (pid, process) in self.system.processes() {
            let identity = ProcessIdentity {
                pid: pid.as_u32(),
                start_time: process.start_time(),
            };
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
            processes.push(ProcessInfo {
                identity,
                name: process.name().to_string_lossy().into_owned(),
                command,
                cpu_percent: self
                    .previous_processes
                    .contains(&identity)
                    .then(|| process.cpu_usage()),
                memory_bytes: Some(process.memory()),
                user,
                status: Some(process.status().to_string()),
                traffic: None,
            });
            current.insert(identity);
        }
        self.previous_processes = current;
        processes.sort_by_key(|process| process.identity.pid);
        processes
    }
}

impl Default for ProcessCollector {
    fn default() -> Self {
        Self::new()
    }
}
