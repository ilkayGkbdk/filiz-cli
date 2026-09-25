use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::runtime::{Control, StopGuard};
use super::system::byte_rate;
use crate::model::TrafficRate;
use crate::state::{CollectorUpdate, ProcessTraffic, Source};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NettopRow {
    pub pid: u32,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

/// Incremental parser for `nettop -P -x -L 0` CSV output.
#[derive(Debug, Default)]
pub struct NettopParser {
    columns: Option<(usize, usize)>,
    current: Vec<NettopRow>,
    started: bool,
}

impl NettopParser {
    /// Feed one line; returns the finished block when the next header starts.
    pub fn push_line(&mut self, line: &str) -> Option<Vec<NettopRow>> {
        if line.starts_with("time,") {
            let names: Vec<&str> = line.split(',').map(str::trim).collect();
            let position = |name: &str| names.iter().position(|column| *column == name);
            self.columns = position("bytes_in").zip(position("bytes_out"));
            let finished = self.started.then(|| std::mem::take(&mut self.current));
            self.started = true;
            return finished;
        }
        let (rx_index, tx_index) = self.columns?;
        let fields: Vec<&str> = line.split(',').collect();
        let pid = fields.get(1)?.rsplit_once('.')?.1.trim().parse().ok()?;
        let bytes_in = fields.get(rx_index)?.trim().parse().ok()?;
        let bytes_out = fields.get(tx_index)?.trim().parse().ok()?;
        self.current.push(NettopRow {
            pid,
            bytes_in,
            bytes_out,
        });
        None
    }
}

/// Turns cumulative per-process byte counters into rates.
#[derive(Debug, Default)]
pub struct TrafficTracker {
    previous: HashMap<u32, (u64, u64)>,
    previous_at: Option<Instant>,
}

impl TrafficTracker {
    pub fn update(&mut self, rows: &[NettopRow], now: Instant) -> Vec<ProcessTraffic> {
        let elapsed = self
            .previous_at
            .map(|at| now.saturating_duration_since(at).as_secs_f64());
        let mut traffic = Vec::new();
        for row in rows {
            let Some(((rx0, tx0), seconds)) = self.previous.get(&row.pid).copied().zip(elapsed)
            else {
                continue;
            };
            if let (Some(rx), Some(tx)) = (
                byte_rate(rx0, row.bytes_in, seconds),
                byte_rate(tx0, row.bytes_out, seconds),
            ) {
                if rx + tx > 0.0 {
                    traffic.push(ProcessTraffic {
                        pid: row.pid,
                        rate: TrafficRate { rx, tx },
                    });
                }
            }
        }
        self.previous = rows
            .iter()
            .map(|row| (row.pid, (row.bytes_in, row.bytes_out)))
            .collect();
        self.previous_at = Some(now);
        traffic
    }
}

#[derive(Clone, Debug)]
pub struct TrafficConfig {
    pub program: String,
    pub args: Vec<String>,
    pub retry_delay: Duration,
    pub max_restarts: u32,
}

impl Default for TrafficConfig {
    fn default() -> Self {
        Self {
            program: "/usr/bin/nettop".into(),
            args: ["-P", "-x", "-L", "0", "-s", "2"]
                .map(String::from)
                .to_vec(),
            retry_delay: Duration::from_secs(10),
            max_restarts: 3,
        }
    }
}

type ChildSlot = Arc<Mutex<Option<Child>>>;

fn kill(slot: &ChildSlot) {
    if let Some(mut child) = slot.lock().expect("child slot").take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub fn spawn(
    config: TrafficConfig,
    updates: Sender<CollectorUpdate>,
) -> (Sender<Control>, JoinHandle<()>, Box<dyn FnOnce() + Send>) {
    let (control_tx, control_rx) = mpsc::channel();
    let slot: ChildSlot = Arc::new(Mutex::new(None));
    let worker_slot = slot.clone();
    let handle = std::thread::Builder::new()
        .name("filiz-traffic".into())
        .spawn(move || run(config, updates, control_rx, worker_slot))
        .expect("spawn traffic thread");
    (control_tx, handle, Box::new(move || kill(&slot)))
}

/// Drains all pending control messages: a queued `Refresh` must not hide a
/// `Stop` sitting behind it in the channel.
fn stop_requested(control: &Receiver<Control>) -> bool {
    loop {
        match control.try_recv() {
            Ok(Control::Stop) | Err(TryRecvError::Disconnected) => return true,
            Ok(Control::Refresh) => continue,
            Err(TryRecvError::Empty) => return false,
        }
    }
}

fn run(
    config: TrafficConfig,
    updates: Sender<CollectorUpdate>,
    control: Receiver<Control>,
    slot: ChildSlot,
) {
    let mut guard = StopGuard::new(Source::Traffic, updates.clone());
    let mut failures = 0;
    loop {
        let spawned = Command::new(&config.program)
            .args(&config.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let message = match spawned {
            Ok(mut child) => {
                let stdout = child.stdout.take().expect("piped stdout");
                *slot.lock().expect("child slot") = Some(child);
                // Stop is sent before child killers run, so checking after storing the child is race-free.
                if stop_requested(&control) {
                    kill(&slot);
                    break;
                }
                let delivered = stream(BufReader::new(stdout), &updates);
                kill(&slot);
                if delivered {
                    failures = 0;
                }
                "nettop exited".to_owned()
            }
            Err(error) => format!("nettop unavailable: {error}"),
        };
        if stop_requested(&control) {
            break;
        }
        if updates
            .send(CollectorUpdate::Failed {
                source: Source::Traffic,
                message,
            })
            .is_err()
        {
            break;
        }
        failures += 1;
        if failures > config.max_restarts {
            break;
        }
        match control.recv_timeout(config.retry_delay) {
            Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Ok(Control::Refresh) | Err(RecvTimeoutError::Timeout) => {}
        }
    }
    guard.disarm();
}

/// Streams nettop output until it exits or the channel closes.
/// Returns `true` if at least one block was delivered, so the caller can
/// reset the restart-failure counter on a genuinely working run.
fn stream(reader: impl BufRead, updates: &Sender<CollectorUpdate>) -> bool {
    let mut parser = NettopParser::default();
    let mut tracker = TrafficTracker::default();
    let mut delivered = false;
    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        if let Some(rows) = parser.push_line(&line) {
            let traffic = tracker.update(&rows, Instant::now());
            if updates.send(CollectorUpdate::Traffic(traffic)).is_err() {
                break;
            }
            delivered = true;
        }
    }
    delivered
}
