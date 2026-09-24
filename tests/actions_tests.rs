#[allow(dead_code)]
#[path = "../src/actions.rs"]
mod actions;
#[allow(dead_code)]
#[path = "../src/model.rs"]
mod model;

use actions::{ActionError, ProcessAction};
use model::{ActionKind, PendingAction, ProcessIdentity};

fn selected() -> ProcessIdentity {
    ProcessIdentity {
        pid: 42,
        start_time: 100,
    }
}

#[test]
fn action_requires_an_explicit_confirmation_transition() {
    let pending = PendingAction::new(selected(), ActionKind::Terminate);
    assert_eq!(pending.identity(), selected());
    assert_eq!(pending.kind(), ActionKind::Terminate);
    let confirmed = pending.confirm();
    assert_eq!(confirmed.identity(), selected());
    assert_eq!(confirmed.kind(), ActionKind::Terminate);
}

#[test]
fn cancelled_action_has_no_executable_result() {
    let pending = PendingAction::new(selected(), ActionKind::Kill);
    let cancelled = pending.cancel();
    assert_eq!(cancelled.identity(), selected());
    assert_eq!(cancelled.kind(), ActionKind::Kill);
}

#[test]
fn action_errors_have_readable_messages() {
    assert!(ActionError::PermissionDenied
        .to_user_message()
        .contains("permission"));
    assert!(ActionError::ProcessNotFound
        .to_user_message()
        .contains("no longer"));
    assert!(ActionError::SelectionChanged
        .to_user_message()
        .contains("changed"));
    assert!(ActionError::OsFailure("failure".to_owned())
        .to_user_message()
        .contains("failure"));
}

#[test]
fn confirmed_action_rejects_a_missing_process_without_signalling() {
    let confirmed = PendingAction::new(
        ProcessIdentity {
            pid: u32::MAX,
            start_time: 1,
        },
        ActionKind::Terminate,
    )
    .confirm();
    assert_eq!(
        ProcessAction::execute(confirmed),
        Err(ActionError::ProcessNotFound)
    );
}

#[test]
fn confirmed_action_rejects_self_target_before_identity_lookup() {
    let current_pid = sysinfo::get_current_pid().expect("current PID").as_u32();
    let confirmed = PendingAction::new(
        ProcessIdentity {
            pid: current_pid,
            start_time: 0,
        },
        ActionKind::Kill,
    )
    .confirm();
    assert_eq!(
        ProcessAction::execute(confirmed),
        Err(ActionError::SelfTarget)
    );
}

#[test]
fn invalid_pid_is_a_typed_error_without_signalling() {
    assert_eq!(
        ProcessAction::terminate(0),
        Err(ActionError::ProcessNotFound)
    );
    assert_eq!(
        ProcessAction::kill(u32::MAX),
        Err(ActionError::ProcessNotFound)
    );
}

#[test]
fn self_targeted_actions_return_a_typed_error_without_signalling() {
    let current_pid = std::process::id();
    for kind in [ActionKind::Terminate, ActionKind::Kill] {
        let confirmed = PendingAction::new(
            ProcessIdentity {
                pid: current_pid,
                start_time: 0,
            },
            kind,
        )
        .confirm();
        assert_eq!(
            ProcessAction::execute(confirmed),
            Err(ActionError::SelfTarget)
        );
    }
    assert_eq!(
        ProcessAction::terminate(current_pid),
        Err(ActionError::SelfTarget)
    );
    assert_eq!(
        ProcessAction::kill(current_pid),
        Err(ActionError::SelfTarget)
    );
    assert!(ActionError::SelfTarget.to_user_message().contains("itself"));
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "manual process signal check"]
fn manual_terminate_harmless_user_owned_process() {
    use std::process::Command;
    use std::time::Duration;
    use sysinfo::{Pid, ProcessesToUpdate, System};

    let mut child = Command::new("/bin/sleep")
        .arg("30")
        .spawn()
        .expect("spawn sleep");
    let pid = Pid::from_u32(child.id());
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let start_time = system.process(pid).expect("sleep process").start_time();
    let confirmed = PendingAction::new(
        ProcessIdentity {
            pid: child.id(),
            start_time,
        },
        ActionKind::Terminate,
    )
    .confirm();
    ProcessAction::execute(confirmed).expect("terminate sleep");
    std::thread::sleep(Duration::from_millis(100));
    assert!(child.try_wait().expect("wait for sleep").is_some());
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "manual permission check"]
fn manual_root_owned_process_returns_readable_permission_error() {
    // launchd (PID 1) is root owned; skip if the test itself has elevated privileges.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let error = ProcessAction::terminate(1).expect_err("cannot terminate launchd");
    assert_eq!(error, ActionError::PermissionDenied);
    assert!(error.to_user_message().contains("permission"));
}
