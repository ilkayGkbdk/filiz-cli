#[cfg(target_os = "macos")]
use std::process::Command;

use sysinfo::Components;

use crate::state::{Battery, PlatformSample};

#[derive(Clone, Debug, PartialEq)]
pub struct BatteryReading {
    pub percent: f64,
    pub power_source: Option<String>,
    pub charging: Option<bool>,
}

pub fn parse_cpu_usage(output: &str) -> (Option<f64>, Option<f64>) {
    let line = output.lines().find(|line| line.contains("CPU usage:"));
    let Some(line) = line else {
        return (None, None);
    };
    let values = line
        .split(':')
        .nth(1)
        .unwrap_or_default()
        .split(',')
        .filter_map(|part| part.split('%').next()?.trim().parse::<f64>().ok())
        .collect::<Vec<_>>();
    (values.first().copied(), values.get(1).copied())
}

/// Parse human-readable `pmset -g batt` output without assuming a battery exists.
pub fn parse_battery(output: &str) -> Option<BatteryReading> {
    let source = output
        .lines()
        .next()
        .and_then(|line| line.split('\'').nth(1))
        .map(str::trim)
        .filter(|source| !source.is_empty())
        .map(str::to_owned);
    let battery_line = output.lines().find(|line| line.contains("%"))?;
    let before_percent = battery_line.split('%').next()?;
    let digits_reversed: String = before_percent
        .chars()
        .rev()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    let percent: u8 = digits_reversed
        .chars()
        .rev()
        .collect::<String>()
        .parse()
        .ok()?;
    if percent > 100 {
        return None;
    }
    let charging = match battery_line.split(';').nth(1).map(str::trim) {
        Some("charging" | "finishing charge") => Some(true),
        Some("discharging" | "charged") => Some(false),
        _ => None,
    };
    Some(BatteryReading {
        percent: f64::from(percent),
        power_source: source,
        charging,
    })
}

pub struct MacOsCollector {
    components: Components,
}

impl MacOsCollector {
    pub fn new() -> Self {
        Self {
            components: Components::new(),
        }
    }

    pub fn sample(&mut self) -> PlatformSample {
        let mut sample = PlatformSample::default();
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("/usr/bin/top")
                .args(["-l", "1", "-n", "0"])
                .output()
            {
                if output.status.success() {
                    let (user, system) = parse_cpu_usage(&String::from_utf8_lossy(&output.stdout));
                    sample.cpu_user = user;
                    sample.cpu_system = system;
                }
            }
            if let Ok(output) = Command::new("/usr/bin/pmset").args(["-g", "batt"]).output() {
                if output.status.success() {
                    sample.battery =
                        parse_battery(&String::from_utf8_lossy(&output.stdout)).map(|reading| {
                            Battery {
                                percent: reading.percent,
                                charging: reading.charging,
                                power_source: reading.power_source,
                            }
                        });
                }
            }
        }
        self.components.refresh_list();
        let readable = |component: &&sysinfo::Component| {
            component.temperature().is_finite() && component.temperature() > 0.0
        };
        sample.temperature = self
            .components
            .list()
            .iter()
            .filter(readable)
            .find(|component| component.label().to_ascii_lowercase().contains("cpu"))
            .or_else(|| self.components.list().iter().find(readable))
            .map(|component| f64::from(component.temperature()));
        sample
    }
}

impl Default for MacOsCollector {
    fn default() -> Self {
        Self::new()
    }
}
