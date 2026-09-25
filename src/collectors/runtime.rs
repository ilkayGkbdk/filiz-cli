use std::sync::mpsc::{self, RecvTimeoutError, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::state::{CollectorUpdate, Source};

use super::macos::MacOsCollector;
use super::processes::ProcessCollector;
use super::system::SystemCollector;

pub type Job = Box<dyn FnMut() -> CollectorUpdate + Send>;

pub struct WorkerSpec {
    pub source: Source,
    pub interval: Duration,
    pub job: Job,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Refresh,
    Stop,
}

pub struct CollectorRuntime {
    controls: Vec<Sender<Control>>,
    handles: Vec<JoinHandle<()>>,
    child_killers: Vec<Box<dyn FnOnce() + Send>>,
}

/// Sends `Stopped` when a worker thread ends without a requested stop (e.g. panic).
pub(crate) struct StopGuard {
    source: Source,
    updates: Sender<CollectorUpdate>,
    armed: bool,
}

impl StopGuard {
    pub(crate) fn new(source: Source, updates: Sender<CollectorUpdate>) -> Self {
        Self {
            source,
            updates,
            armed: true,
        }
    }

    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for StopGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.updates.send(CollectorUpdate::Stopped(self.source));
        }
    }
}

impl CollectorRuntime {
    pub fn start(updates: Sender<CollectorUpdate>) -> Self {
        let mut system = SystemCollector::new();
        let mut processes = ProcessCollector::new();
        let mut platform = MacOsCollector::new();
        let mut runtime = Self::with_workers(
            vec![
                WorkerSpec {
                    source: Source::System,
                    interval: Duration::from_secs(2),
                    job: Box::new(move || {
                        let mut sample = system.sample();
                        sample.processes = processes.sample();
                        CollectorUpdate::System(sample)
                    }),
                },
                WorkerSpec {
                    source: Source::Platform,
                    interval: Duration::from_secs(5),
                    job: Box::new(move || CollectorUpdate::Platform(platform.sample())),
                },
            ],
            updates.clone(),
        );
        let (control, handle, killer) =
            super::traffic::spawn(super::traffic::TrafficConfig::default(), updates);
        runtime.adopt(control, handle);
        runtime.add_child_killer(killer);
        runtime
    }

    pub fn with_workers(workers: Vec<WorkerSpec>, updates: Sender<CollectorUpdate>) -> Self {
        let mut runtime = Self {
            controls: Vec::new(),
            handles: Vec::new(),
            child_killers: Vec::new(),
        };
        for worker in workers {
            let (control_tx, control_rx) = mpsc::channel();
            let updates = updates.clone();
            let handle = std::thread::Builder::new()
                .name(format!("filiz-{:?}", worker.source).to_lowercase())
                .spawn(move || run_worker(worker, updates, control_rx))
                .expect("spawn collector thread");
            runtime.controls.push(control_tx);
            runtime.handles.push(handle);
        }
        runtime
    }

    pub fn refresh(&self) {
        for control in &self.controls {
            let _ = control.send(Control::Refresh);
        }
    }

    /// Register a thread spawned outside `with_workers` (see traffic worker).
    pub fn adopt(&mut self, control: Sender<Control>, handle: JoinHandle<()>) {
        self.controls.push(control);
        self.handles.push(handle);
    }

    /// Register a callback that kills a child process on shutdown.
    pub fn add_child_killer(&mut self, killer: Box<dyn FnOnce() + Send>) {
        self.child_killers.push(killer);
    }
}

fn run_worker(
    mut worker: WorkerSpec,
    updates: Sender<CollectorUpdate>,
    control: mpsc::Receiver<Control>,
) {
    let mut guard = StopGuard::new(worker.source, updates.clone());
    loop {
        if updates.send((worker.job)()).is_err() {
            break;
        }
        let first = control.recv_timeout(worker.interval);
        if matches!(first, Err(RecvTimeoutError::Timeout)) {
            continue;
        }
        let mut stop = matches!(
            first,
            Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected)
        );
        // Drain any additional queued control messages so a backlog of
        // `Refresh` requests (e.g. from key repeat) doesn't delay a `Stop`
        // or force multiple redundant jobs to run before shutdown.
        loop {
            match control.try_recv() {
                Ok(Control::Stop) | Err(TryRecvError::Disconnected) => {
                    stop = true;
                    break;
                }
                Ok(Control::Refresh) => {}
                Err(TryRecvError::Empty) => break,
            }
        }
        if stop {
            break;
        }
    }
    guard.disarm();
}

impl Drop for CollectorRuntime {
    fn drop(&mut self) {
        for control in &self.controls {
            let _ = control.send(Control::Stop);
        }
        for killer in self.child_killers.drain(..) {
            killer();
        }
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}
