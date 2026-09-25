use std::collections::{HashMap, VecDeque};

use crate::state::SystemState;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SeriesKey {
    Cpu,
    Memory,
    Disk,
    NetRx(String),
    NetTx(String),
}

#[derive(Clone, Debug)]
pub struct History {
    series: HashMap<SeriesKey, VecDeque<u64>>,
    capacity: usize,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            series: HashMap::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn push(&mut self, key: SeriesKey, value: Option<f64>) {
        let Some(value) = value else {
            return;
        };
        let series = self.series.entry(key).or_default();
        series.push_back(value.clamp(0.0, u64::MAX as f64) as u64);
        while series.len() > self.capacity {
            series.pop_front();
        }
    }

    pub fn series(&self, key: &SeriesKey) -> Vec<u64> {
        self.series
            .get(key)
            .map(|series| series.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn len(&self, key: &SeriesKey) -> usize {
        self.series.get(key).map_or(0, VecDeque::len)
    }

    /// Record one sample per series from the latest system state.
    pub fn record(&mut self, state: &SystemState) {
        self.push(SeriesKey::Cpu, state.cpu.usage);
        self.push(SeriesKey::Memory, state.memory.usage_percent());
        self.push(
            SeriesKey::Disk,
            state.primary_disk().and_then(|disk| disk.usage_percent()),
        );
        for interface in &state.interfaces {
            self.push(SeriesKey::NetRx(interface.name.clone()), interface.rx_rate);
            self.push(SeriesKey::NetTx(interface.name.clone()), interface.tx_rate);
        }
        self.series.retain(|key, _| match key {
            SeriesKey::NetRx(name) | SeriesKey::NetTx(name) => state
                .interfaces
                .iter()
                .any(|interface| &interface.name == name),
            _ => true,
        });
    }
}
