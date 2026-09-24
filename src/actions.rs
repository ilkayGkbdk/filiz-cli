use std::fmt;
use std::io;

use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::model::{ActionKind, ConfirmedAction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionError {
    PermissionDenied,
    ProcessNotFound,
    SelectionChanged,
    SelfTarget,
    OsFailure(String),
}

impl ActionError {
    pub fn to_user_message(&self) -> String {
        match self {
            Self::PermissionDenied => "Process permission denied.".to_owned(),
            Self::ProcessNotFound => "The process is no longer running.".to_owned(),
            Self::SelectionChanged => "The selected process changed; select it again.".to_owned(),
            Self::SelfTarget => "Filiz cannot send a signal to itself.".to_owned(),
            Self::OsFailure(reason) => format!("Process action failed: {reason}"),
        }
    }
}

impl fmt::Display for ActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_user_message())
    }
}

impl std::error::Error for ActionError {}

pub struct ProcessAction;

impl ProcessAction {
    /// Execute only after the UI turns a pending selection into a confirmed action.
    pub fn execute(action: ConfirmedAction) -> Result<(), ActionError> {
        let identity = action.identity();
        if identity.pid == std::process::id() {
            return Err(ActionError::SelfTarget);
        }
        let pid = Pid::from_u32(identity.pid);
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
        let current = system.process(pid).ok_or(ActionError::ProcessNotFound)?;
        if current.start_time() != identity.start_time {
            return Err(ActionError::SelectionChanged);
        }

        match action.kind() {
            ActionKind::Terminate => Self::terminate(identity.pid),
            ActionKind::Kill => Self::kill(identity.pid),
        }
    }

    pub fn terminate(pid: u32) -> Result<(), ActionError> {
        send_signal(pid, libc::SIGTERM)
    }

    pub fn kill(pid: u32) -> Result<(), ActionError> {
        send_signal(pid, libc::SIGKILL)
    }
}

fn send_signal(pid: u32, signal: libc::c_int) -> Result<(), ActionError> {
    if pid == std::process::id() {
        return Err(ActionError::SelfTarget);
    }
    let pid = libc::pid_t::try_from(pid).map_err(|_| ActionError::ProcessNotFound)?;
    if pid <= 0 {
        return Err(ActionError::ProcessNotFound);
    }
    // SAFETY: kill accepts a positive PID and a valid POSIX signal constant.
    if unsafe { libc::kill(pid, signal) } == 0 {
        Ok(())
    } else {
        Err(map_os_error(io::Error::last_os_error()))
    }
}

fn map_os_error(error: io::Error) -> ActionError {
    match error.raw_os_error() {
        Some(libc::EPERM) | Some(libc::EACCES) => ActionError::PermissionDenied,
        Some(libc::ESRCH) => ActionError::ProcessNotFound,
        _ => ActionError::OsFailure(error.to_string()),
    }
}
