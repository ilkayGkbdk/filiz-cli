use std::process::Command;

use sysinfo::Components;

use crate::model::{
    CollectorData, CollectorResult, CollectorWarning, MacOsMetrics, ResourceMetric,
};

use super::Collector;

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

    pub fn collect(&mut self) -> MacOsMetrics {
        let mut result = MacOsMetrics::default();
        #[cfg(target_os = "macos")]
        {
            match Command::new("/usr/bin/top")
                .args(["-l", "1", "-n", "0"])
                .output()
            {
                Ok(output) if output.status.success() => {
                    let (user, system) = parse_cpu_usage(&String::from_utf8_lossy(&output.stdout));
                    result.cpu_user_percent = user;
                    result.cpu_system_percent = system;
                }
                _ => {}
            }
            match Command::new("/usr/bin/pmset").args(["-g", "batt"]).output() {
                Ok(output) if output.status.success() => {
                    match parse_battery(&String::from_utf8_lossy(&output.stdout)) {
                        Some(battery) => {
                            result.battery_percent = Some(battery.percent);
                            result.battery_power_source = battery.power_source;
                            result.battery_charging = battery.charging;
                            if result.battery_power_source.is_none() {
                                warn(&mut result, "battery", "battery power source unavailable");
                            }
                            if result.battery_charging.is_none() {
                                warn(&mut result, "battery", "battery charging state unavailable");
                            }
                        }
                        None => warn(&mut result, "battery", "battery data unavailable"),
                    }
                }
                Ok(_) => warn(&mut result, "battery", "pmset could not read battery data"),
                Err(error) => warn(
                    &mut result,
                    "battery",
                    &format!("pmset unavailable: {error}"),
                ),
            }

            self.components.refresh_list();
            let candidate = self
                .components
                .list()
                .iter()
                .filter(|component| {
                    component.temperature().is_finite() && component.temperature() > 0.0
                })
                .find(|component| component.label().to_ascii_lowercase().contains("cpu"))
                .or_else(|| {
                    self.components.list().iter().find(|component| {
                        component.temperature().is_finite() && component.temperature() > 0.0
                    })
                });
            result.temperature_celsius =
                candidate.map(|component| f64::from(component.temperature()));
            if result.temperature_celsius.is_none() {
                warn(&mut result, "temperature", "temperature sensor unavailable");
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = &self.components;
            warn(
                &mut result,
                "battery",
                "battery data is available only on macOS",
            );
            warn(
                &mut result,
                "temperature",
                "temperature data is available only on macOS",
            );
        }
        result
    }
}

impl Default for MacOsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for MacOsCollector {
    fn collect(&mut self) -> CollectorResult {
        let values = MacOsCollector::collect(self);
        Ok(CollectorData {
            metrics: vec![
                ResourceMetric {
                    name: "cpu.user".to_owned(),
                    value: values.cpu_user_percent,
                    unit: "%".to_owned(),
                },
                ResourceMetric {
                    name: "cpu.system".to_owned(),
                    value: values.cpu_system_percent,
                    unit: "%".to_owned(),
                },
                ResourceMetric {
                    name: "battery.percent".to_owned(),
                    value: values.battery_percent,
                    unit: "%".to_owned(),
                },
                ResourceMetric {
                    name: "battery.charging".to_owned(),
                    value: values
                        .battery_charging
                        .map(|charging| if charging { 1.0 } else { 0.0 }),
                    unit: "bool".to_owned(),
                },
                ResourceMetric {
                    name: "temperature.celsius".to_owned(),
                    value: values.temperature_celsius,
                    unit: "°C".to_owned(),
                },
            ],
            warnings: values.warnings,
            ..Default::default()
        })
    }
}

fn warn(result: &mut MacOsMetrics, collector: &str, message: &str) {
    result.warnings.push(CollectorWarning {
        collector: collector.to_owned(),
        message: message.to_owned(),
    });
}
