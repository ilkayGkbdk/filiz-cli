# Filiz Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Collector'ları arka plan thread'lerine taşıyıp typed durum modeli, bağlama duyarlı keymap, bileşen/workspace yapısı ve mouse hit-test altyapısını kurmak; görünümü koruyup UI v2 hatalarını düzeltmek.

**Architecture:** Worker thread'leri `CollectorUpdate` mesajlarını `mpsc` kanalına yollar; ana döngü bunları saf `SystemState::apply` ile birleştirir. Girdi, tek bir `BINDINGS` tablosu üzerinden bağlam yığınıyla `Action`'a çevrilir; `App::update` saf kalır ve `Effect` döndürür. Render bir `RenderOutput` (viewport yükseklikleri + `HitMap`) üretir; mouse olayları bu haritayla `Action`'a çevrilir.

**Tech Stack:** Rust 2021, ratatui 0.29, crossterm 0.28, sysinfo 0.32, libc, anyhow. Yeni bağımlılık yok.

**Spec:** `docs/superpowers/specs/2026-09-25-filiz-foundation-design.md`

## Global Constraints

- Yeni crate bağımlılığı eklenmez; eşzamanlılık yalnızca `std::thread` ve `std::sync::mpsc`.
- Ölçülemeyen her değer `Option`'dır ve UI'da `N/A` görünür; geçmişe sahte `0` yazılmaz.
- Forest teması mevcut renkleri birebir korur: bg `(9,13,10)`, surface `(19,26,20)`, border `(66,78,56)`, text `(232,235,216)`, muted `(148,158,135)`, accent `(164,191,101)`, ok `(137,207,138)`, warn `(235,194,91)`, danger `(232,117,100)`.
- Ana döngü girdiyi en fazla 50 ms bekler; UI thread'inde hiçbir collector çalışmaz.
- Süreç aksiyonları: açık `Y` onayı, kendini hedefleme koruması, `ProcessIdentity { pid, start_time }` doğrulaması korunur.
- Tuşlar: `Q` çıkış, `1`–`5` ve `←/→` workspace, `Tab`/`Shift+Tab` odak, `S` sıralama, `M` menü, `H`/`L`/`T`, `R` yenile, `F` filtre, `K`/`Shift+K`, `Y`/`N`. `C`/`M` ile sıralama kaldırılır.
- UI metinleri mevcut dil olan İngilizcede kalır.
- Her task sonunda şu komutlar geçer: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- Çalışma dalı: `feat/filiz-foundation`.

## Review Focus

- Liste kaydırılmışken terminalin 20×8'e küçülmesi veya listenin kısalması → panic yok, seçim ve offset sınır içinde kalır (Task 5 `list_state_survives_shrinking_list_and_viewport`, Task 7 `every_workspace_density_and_size_renders_with_scrolled_lists`).
- Filtre yazarken büyük harf ve Türkçe karakterler (`İ`, `ş`, `Q`) → karakter olarak eklenir, uygulamadan çıkmaz (Task 5 `filter_accepts_uppercase_and_turkish_characters`).
- Seçili interface (ör. `utun3`) kaybolduğunda → seçim en yakın geçerli satıra sıkışır, o interface'in geçmiş serisi silinir (Task 2 `history_drops_series_for_vanished_interfaces`, Task 6 `interface_selection_clamps_when_interface_disappears`).
- İçinde nokta olan nettop süreç adları (`com.apple.WebKit.Networking.812`) → pid doğru ayrıştırılır (Task 9 `parser_takes_pid_after_last_dot`).
- Yavaş bir collector çalışırken tuşa basma ve `Q` → girdi beklemeden işlenir, çıkış en geç bir iş süresi kadar gecikir (Task 4 `pump_never_blocks_on_slow_worker`, `runtime_drop_joins_workers`).

## File Structure

```text
src/
  lib.rs                 modül kayıtları (state, history, input eklenir)
  main.rs                terminal guard, panic hook, splash, runtime başlatma
  model.rs               ProcessInfo (+traffic), ProcessIdentity, aksiyon tipleri, filtre/sıralama
  state.rs               NEW  Source, *Stats, SystemSample, PlatformSample, CollectorUpdate, SystemState::apply
  history.rs             NEW  SeriesKey, History
  actions.rs             değişmez
  app.rs                 App, App::update, pump, run döngüsü
  collectors/
    mod.rs               modül kayıtları, sample yardımcıları
    system.rs            SystemCollector::sample -> SystemSample
    processes.rs         ProcessCollector::sample -> Vec<ProcessInfo>
    macos.rs             MacOsCollector::sample -> PlatformSample
    runtime.rs           NEW  CollectorRuntime, worker döngüsü, Control
    traffic.rs           NEW  NettopParser, TrafficTracker, traffic worker
  input/
    mod.rs               NEW
    action.rs            NEW  Action, Effect
    keymap.rs            NEW  Context, KeySpec, Binding, BINDINGS, resolve, hints
    list.rs              NEW  ListState
  ui/
    mod.rs               render -> RenderOutput, chrome + workspace yönlendirme
    format.rs            NEW  bytes, rate, percent, uptime
    theme.rs             rol token'lı Palette
    state.rs             Workspace, PanelId, LayoutDensity, UiState
    hit.rs               NEW  HitMap, HitTarget, mouse_action
    splash.rs            NEW  raw-mode splash text
    components/          NEW  status.rs, card.rs, table.rs, modal.rs, footer.rs
    workspaces/          NEW  mod.rs, overview.rs, processes.rs, network.rs, disks.rs, more.rs
    widgets.rs           Task 6'da silinir
tests/
  format_tests.rs        NEW
  theme_tests.rs         NEW
  state_tests.rs         NEW
  runtime_tests.rs       NEW
  keymap_tests.rs        NEW
  app_tests.rs           NEW
  hit_tests.rs           NEW
  traffic_tests.rs       NEW
  collector_tests.rs     `filiz::` yollarına geçer
  ui_tests.rs            yeni modele geçer
```

---

### Task 1: Format yardımcıları ve tema token'ları

Davranış değişmez; kopya biçimlendirme kodu tek yere toplanır ve palet rol token'larına geçer.

**Files:**
- Create: `src/ui/format.rs`
- Modify: `src/ui/theme.rs` (tamamı), `src/ui/mod.rs:1-3,19`, `src/ui/widgets.rs` (palet alan adları, `bytes`/`rate`/`percent`/`format_uptime` silinir)
- Test: `tests/format_tests.rs`, `tests/theme_tests.rs`

**Interfaces:**
- Produces: `filiz::ui::format::{bytes(u64) -> String, rate(f64) -> String, percent(Option<f64>) -> String, uptime(u64) -> String}`
- Produces: `filiz::ui::theme::Palette` alanları `bg, surface, surface_alt, border, border_focus, text, text_muted, accent, accent_fg, selection_bg, selection_fg, ok, warn, danger, info, chart_rx, chart_tx: Color` ve `danger_modifier: Modifier`; `Theme::{Forest, Amber, Mono, Solarized}`, `Theme::palette()`, `Theme::next()`, `Theme::label()`, `Theme::ALL`.

- [ ] **Step 1: Write the failing tests**

`tests/format_tests.rs`:

```rust
use filiz::ui::format::{bytes, percent, rate, uptime};

#[test]
fn bytes_scale_through_binary_units() {
    assert_eq!(bytes(0), "0.0B");
    assert_eq!(bytes(1536), "1.5KB");
    assert_eq!(bytes(5 * 1024 * 1024 * 1024), "5.0GB");
}

#[test]
fn rate_clamps_negative_values_and_appends_per_second() {
    assert_eq!(rate(2048.0), "2.0KB/s");
    assert_eq!(rate(-5.0), "0.0B/s");
}

#[test]
fn percent_renders_missing_values_as_na() {
    assert_eq!(percent(Some(61.6)), "62%");
    assert_eq!(percent(None), "N/A");
}

#[test]
fn uptime_switches_to_days_after_24_hours() {
    assert_eq!(uptime(3_700), "1h 1m");
    assert_eq!(uptime(90_061), "1d 1h");
}
```

`tests/theme_tests.rs`:

```rust
use filiz::ui::theme::Theme;
use ratatui::style::{Color, Modifier};

#[test]
fn forest_keeps_existing_colors() {
    let p = Theme::Forest.palette();
    assert_eq!(p.bg, Color::Rgb(9, 13, 10));
    assert_eq!(p.surface, Color::Rgb(19, 26, 20));
    assert_eq!(p.border, Color::Rgb(66, 78, 56));
    assert_eq!(p.text, Color::Rgb(232, 235, 216));
    assert_eq!(p.text_muted, Color::Rgb(148, 158, 135));
    assert_eq!(p.accent, Color::Rgb(164, 191, 101));
    assert_eq!(p.ok, Color::Rgb(137, 207, 138));
    assert_eq!(p.warn, Color::Rgb(235, 194, 91));
    assert_eq!(p.danger, Color::Rgb(232, 117, 100));
}

#[test]
fn every_theme_distinguishes_danger_from_text() {
    for theme in Theme::ALL {
        let p = theme.palette();
        assert!(
            p.danger != p.text || p.danger_modifier.contains(Modifier::REVERSED),
            "{theme:?} danger is indistinguishable"
        );
    }
}

#[test]
fn mono_marks_danger_with_reverse_video() {
    assert!(Theme::Mono
        .palette()
        .danger_modifier
        .contains(Modifier::REVERSED | Modifier::BOLD));
}

#[test]
fn theme_cycle_visits_all_four_themes() {
    let mut theme = Theme::Forest;
    let mut seen = vec![theme];
    for _ in 0..3 {
        theme = theme.next();
        seen.push(theme);
    }
    assert_eq!(seen, Theme::ALL.to_vec());
    assert_eq!(theme.next(), Theme::Forest);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test format_tests --test theme_tests`
Expected: FAIL — `could not find format in ui`, `no field bg on type Palette`.

- [ ] **Step 3: Create `src/ui/format.rs`**

```rust
pub fn bytes(value: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut value = value as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1}{}", UNITS[unit])
}

pub fn rate(value: f64) -> String {
    format!("{}/s", bytes(value.max(0.0) as u64))
}

pub fn percent(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.0}%"))
        .unwrap_or_else(|| "N/A".into())
}

pub fn uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = seconds % 86_400 / 3_600;
    let minutes = seconds % 3_600 / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else {
        format!("{hours}h {minutes}m")
    }
}
```

In `src/ui/mod.rs` add `pub mod format;` after `pub mod state;`.

- [ ] **Step 4: Replace `src/ui/theme.rs` entirely**

```rust
use ratatui::style::{Color, Modifier};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Forest,
    Amber,
    Mono,
    Solarized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bg: Color,
    pub surface: Color,
    pub surface_alt: Color,
    pub border: Color,
    pub border_focus: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub accent_fg: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub ok: Color,
    pub warn: Color,
    pub danger: Color,
    pub info: Color,
    pub chart_rx: Color,
    pub chart_tx: Color,
    pub danger_modifier: Modifier,
}

struct Base {
    bg: (u8, u8, u8),
    surface: (u8, u8, u8),
    border: (u8, u8, u8),
    text: (u8, u8, u8),
    muted: (u8, u8, u8),
    accent: (u8, u8, u8),
    ok: (u8, u8, u8),
    warn: (u8, u8, u8),
    danger: (u8, u8, u8),
    danger_modifier: Modifier,
}

fn rgb((r, g, b): (u8, u8, u8)) -> Color {
    Color::Rgb(r, g, b)
}

impl Base {
    fn palette(self) -> Palette {
        Palette {
            bg: rgb(self.bg),
            surface: rgb(self.surface),
            surface_alt: rgb(self.surface),
            border: rgb(self.border),
            border_focus: rgb(self.accent),
            text: rgb(self.text),
            text_muted: rgb(self.muted),
            accent: rgb(self.accent),
            accent_fg: rgb(self.bg),
            selection_bg: rgb(self.accent),
            selection_fg: rgb(self.bg),
            ok: rgb(self.ok),
            warn: rgb(self.warn),
            danger: rgb(self.danger),
            info: rgb(self.ok),
            chart_rx: rgb(self.ok),
            chart_tx: rgb(self.accent),
            danger_modifier: self.danger_modifier,
        }
    }
}

impl Theme {
    pub const ALL: [Self; 4] = [Self::Forest, Self::Amber, Self::Mono, Self::Solarized];

    pub fn palette(self) -> Palette {
        match self {
            Self::Forest => Base {
                bg: (9, 13, 10),
                surface: (19, 26, 20),
                border: (66, 78, 56),
                text: (232, 235, 216),
                muted: (148, 158, 135),
                accent: (164, 191, 101),
                ok: (137, 207, 138),
                warn: (235, 194, 91),
                danger: (232, 117, 100),
                danger_modifier: Modifier::BOLD,
            },
            Self::Amber => Base {
                bg: (20, 16, 10),
                surface: (31, 25, 16),
                border: (92, 68, 35),
                text: (245, 232, 202),
                muted: (164, 145, 112),
                accent: (235, 181, 74),
                ok: (183, 205, 119),
                warn: (245, 194, 78),
                danger: (232, 117, 84),
                danger_modifier: Modifier::BOLD,
            },
            Self::Mono => Base {
                bg: (12, 12, 12),
                surface: (25, 25, 25),
                border: (82, 82, 82),
                text: (235, 235, 235),
                muted: (158, 158, 158),
                accent: (210, 210, 210),
                ok: (205, 205, 205),
                warn: (235, 235, 235),
                danger: (255, 255, 255),
                danger_modifier: Modifier::BOLD.union(Modifier::REVERSED),
            },
            Self::Solarized => Base {
                bg: (0, 43, 54),
                surface: (7, 54, 66),
                border: (42, 94, 104),
                text: (238, 232, 213),
                muted: (147, 161, 161),
                accent: (181, 137, 0),
                ok: (133, 153, 0),
                warn: (203, 75, 22),
                danger: (220, 50, 47),
                danger_modifier: Modifier::BOLD,
            },
        }
        .palette()
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|theme| *theme == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Forest => "Forest",
            Self::Amber => "Amber",
            Self::Mono => "Mono",
            Self::Solarized => "Solarized",
        }
    }
}
```

The old constants `BACKGROUND … RED` are gone with this replacement.

- [ ] **Step 5: Rename palette fields in UI code**

Run (macOS `sed`):

```bash
sed -i '' \
  -e 's/palette\.background/palette.bg/g' \
  -e 's/palette\.panel/palette.surface/g' \
  -e 's/palette\.muted/palette.text_muted/g' \
  -e 's/palette\.olive/palette.accent/g' \
  -e 's/palette\.green/palette.ok/g' \
  -e 's/palette\.yellow/palette.warn/g' \
  -e 's/palette\.red/palette.danger/g' \
  src/ui/widgets.rs
sed -i '' 's/palette()\.background/palette().bg/' src/ui/mod.rs
```

- [ ] **Step 6: Move formatting out of `widgets.rs`**

Delete `fn percent`, `fn bytes`, `fn rate`, `fn format_uptime` from `src/ui/widgets.rs` (they are at the end of the file, above `fn usage_color`). Add at the top of `widgets.rs`:

```rust
use super::format::{bytes, percent, rate, uptime as format_uptime};
```

- [ ] **Step 7: Run all checks**

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: PASS, including the 8 new tests. Existing UI tests still pass because rendering is unchanged.

- [ ] **Step 8: Commit**

```bash
git add src/ui tests/format_tests.rs tests/theme_tests.rs
git commit -m "refactor: add role-based theme tokens and shared formatting"
```

---

### Task 2: Typed durum modeli ve History

Saf veri katmanı. Henüz hiçbir yerden çağrılmaz; Task 3 bağlar.

**Files:**
- Create: `src/state.rs`, `src/history.rs`
- Modify: `src/lib.rs`, `src/model.rs` (`TrafficRate`, `ProcessInfo.traffic`), `ProcessInfo` literal'leri olan `src/app.rs`, `src/ui/mod.rs`, `src/collectors/processes.rs`, `tests/model_tests.rs`, `tests/ui_tests.rs`
- Test: `tests/state_tests.rs`

**Interfaces:**
- Produces (`filiz::model`): `TrafficRate { rx: f64, tx: f64 }` (`Clone, Copy, Debug, PartialEq`); `ProcessInfo.traffic: Option<TrafficRate>`.
- Produces (`filiz::state`): `Source::{System, Platform, Traffic}`; `CpuStats`, `MemoryStats`, `DiskStats`, `InterfaceStats`, `Battery`, `ProcessTraffic`, `SystemSample`, `PlatformSample`, `CollectorUpdate::{System, Platform, Traffic, Failed { source, message }, Stopped(Source)}`; `SystemState` with `apply(&mut self, CollectorUpdate)`, `has_failures() -> bool`, `issue(Source) -> Option<&str>`, `primary_disk() -> Option<&DiskStats>`, `visible_disks() -> Vec<&DiskStats>`; `is_system_mount(&str) -> bool`.
- Produces (`filiz::history`): `SeriesKey::{Cpu, Memory, Disk, NetRx(String), NetTx(String)}`; `History::new(capacity)`, `push(SeriesKey, Option<f64>)`, `series(&SeriesKey) -> Vec<u64>`, `record(&SystemState)`, `len(&SeriesKey) -> usize`.

- [ ] **Step 1: Add `traffic` to `ProcessInfo`**

In `src/model.rs`, above `pub struct ProcessInfo`:

```rust
/// Per-process network throughput in bytes per second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrafficRate {
    pub rx: f64,
    pub tx: f64,
}
```

Add as the last field of `ProcessInfo`:

```rust
    pub traffic: Option<TrafficRate>,
```

Add `traffic: None,` after every `status: …,` line in `ProcessInfo` literals:

```bash
perl -0pi -e 's/^(\s*)(status: [^\n]*,)\n/$1$2\n$1traffic: None,\n/mg' \
  src/app.rs src/ui/mod.rs src/collectors/processes.rs tests/model_tests.rs tests/ui_tests.rs
cargo test --no-run
```

Expected: compiles.

- [ ] **Step 2: Write the failing tests**

`tests/state_tests.rs`:

```rust
use std::time::Duration;

use filiz::history::{History, SeriesKey};
use filiz::model::{ProcessIdentity, ProcessInfo, TrafficRate};
use filiz::state::{
    is_system_mount, Battery, CollectorUpdate, DiskStats, InterfaceStats, MemoryStats,
    PlatformSample, ProcessTraffic, Source, SystemSample, SystemState,
};

fn process(pid: u32) -> ProcessInfo {
    ProcessInfo {
        identity: ProcessIdentity { pid, start_time: 1 },
        name: format!("p{pid}"),
        command: String::new(),
        cpu_percent: Some(1.0),
        memory_bytes: Some(1),
        user: None,
        status: None,
        traffic: None,
    }
}

fn iface(name: &str, rx: Option<f64>, tx: Option<f64>) -> InterfaceStats {
    InterfaceStats {
        name: name.into(),
        rx_rate: rx,
        tx_rate: tx,
        rx_total: 0,
        tx_total: 0,
        peak_rx: 0.0,
        peak_tx: 0.0,
    }
}

fn disk(mount: &str, used: u64) -> DiskStats {
    DiskStats {
        mount: mount.into(),
        total: 100,
        used,
        free: 100 - used,
        is_system: is_system_mount(mount),
    }
}

fn system_sample() -> SystemSample {
    SystemSample {
        cpu_usage: Some(40.0),
        cores: Some(8),
        load: Some([1.0, 2.0, 3.0]),
        memory: MemoryStats {
            total: Some(100),
            used: Some(25),
            ..Default::default()
        },
        disks: vec![disk("/System/Volumes/VM", 10), disk("/", 70)],
        interfaces: vec![iface("en0", Some(100.0), Some(50.0))],
        processes: vec![process(1), process(2)],
        uptime: Some(Duration::from_secs(60)),
    }
}

fn platform_sample() -> PlatformSample {
    PlatformSample {
        cpu_user: Some(10.0),
        cpu_system: Some(5.0),
        battery: Some(Battery {
            percent: 80.0,
            charging: Some(true),
            power_source: Some("AC Power".into()),
        }),
        temperature: None,
    }
}

#[test]
fn system_update_leaves_platform_fields_untouched() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::Platform(platform_sample()));
    state.apply(CollectorUpdate::System(system_sample()));
    assert_eq!(state.cpu.usage, Some(40.0));
    assert_eq!(state.cpu.user, Some(10.0));
    assert_eq!(state.battery.as_ref().unwrap().percent, 80.0);
    assert_eq!(state.cpu.idle(), Some(60.0));
    assert_eq!(state.memory.usage_percent(), Some(25.0));
}

#[test]
fn missing_sensor_is_not_a_failure() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::Platform(platform_sample()));
    assert_eq!(state.cpu.temperature, None);
    assert!(!state.has_failures());
}

#[test]
fn failure_clears_only_that_source_and_recovers() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::System(system_sample()));
    state.apply(CollectorUpdate::Platform(platform_sample()));
    state.apply(CollectorUpdate::Failed {
        source: Source::Platform,
        message: "top failed".into(),
    });
    assert_eq!(state.cpu.user, None);
    assert!(state.battery.is_none());
    assert_eq!(state.cpu.usage, Some(40.0));
    assert_eq!(state.issue(Source::Platform), Some("top failed"));
    assert!(state.has_failures());

    state.apply(CollectorUpdate::Platform(platform_sample()));
    assert!(!state.has_failures());
}

#[test]
fn stopped_worker_is_reported_as_issue() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::Stopped(Source::System));
    assert_eq!(state.issue(Source::System), Some("data stream stopped"));
}

#[test]
fn traffic_is_attached_to_matching_processes_across_system_updates() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::System(system_sample()));
    state.apply(CollectorUpdate::Traffic(vec![ProcessTraffic {
        pid: 2,
        rate: TrafficRate { rx: 10.0, tx: 5.0 },
    }]));
    let find = |state: &SystemState, pid| {
        state
            .processes
            .iter()
            .find(|p| p.identity.pid == pid)
            .unwrap()
            .traffic
    };
    assert_eq!(find(&state, 2), Some(TrafficRate { rx: 10.0, tx: 5.0 }));
    assert_eq!(find(&state, 1), None);

    state.apply(CollectorUpdate::System(system_sample()));
    assert_eq!(find(&state, 2), Some(TrafficRate { rx: 10.0, tx: 5.0 }));
}

#[test]
fn system_volumes_are_hidden_and_root_is_primary() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::System(system_sample()));
    assert_eq!(state.primary_disk().unwrap().mount, "/");
    let visible: Vec<_> = state.visible_disks().iter().map(|d| d.mount.clone()).collect();
    assert_eq!(visible, vec!["/".to_string()]);
    assert!(is_system_mount("/private/var/vm"));
    assert!(!is_system_mount("/Volumes/USB"));
}

#[test]
fn history_skips_missing_values_and_respects_capacity() {
    let mut history = History::new(3);
    for value in [Some(1.0), None, Some(2.0), Some(3.0), Some(4.0)] {
        history.push(SeriesKey::Cpu, value);
    }
    assert_eq!(history.series(&SeriesKey::Cpu), vec![2, 3, 4]);
}

#[test]
fn history_records_rx_and_tx_separately_per_interface() {
    let mut state = SystemState::default();
    state.apply(CollectorUpdate::System(system_sample()));
    let mut history = History::new(60);
    history.record(&state);
    assert_eq!(history.series(&SeriesKey::NetRx("en0".into())), vec![100]);
    assert_eq!(history.series(&SeriesKey::NetTx("en0".into())), vec![50]);
    assert_eq!(history.series(&SeriesKey::Cpu), vec![40]);
    assert_eq!(history.series(&SeriesKey::Disk), vec![70]);
}

#[test]
fn history_drops_series_for_vanished_interfaces() {
    let mut state = SystemState::default();
    let mut sample = system_sample();
    sample.interfaces.push(iface("utun3", Some(1.0), Some(1.0)));
    state.apply(CollectorUpdate::System(sample));
    let mut history = History::new(60);
    history.record(&state);
    assert_eq!(history.len(&SeriesKey::NetRx("utun3".into())), 1);

    state.apply(CollectorUpdate::System(system_sample()));
    history.record(&state);
    assert_eq!(history.len(&SeriesKey::NetRx("utun3".into())), 0);
    assert_eq!(history.len(&SeriesKey::NetRx("en0".into())), 2);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test state_tests`
Expected: FAIL — `unresolved import filiz::state`.

- [ ] **Step 4: Create `src/state.rs`**

```rust
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use crate::model::{ProcessInfo, TrafficRate};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    System,
    Platform,
    Traffic,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CpuStats {
    pub usage: Option<f64>,
    pub user: Option<f64>,
    pub system: Option<f64>,
    pub cores: Option<usize>,
    pub load: Option<[f64; 3]>,
    pub temperature: Option<f64>,
}

impl CpuStats {
    pub fn idle(&self) -> Option<f64> {
        self.usage.map(|usage| (100.0 - usage).max(0.0))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemoryStats {
    pub total: Option<u64>,
    pub used: Option<u64>,
    pub available: Option<u64>,
    pub free: Option<u64>,
    pub swap_used: Option<u64>,
    pub swap_total: Option<u64>,
}

impl MemoryStats {
    pub fn usage_percent(&self) -> Option<f64> {
        match (self.used, self.total) {
            (Some(used), Some(total)) if total > 0 => Some(used as f64 / total as f64 * 100.0),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiskStats {
    pub mount: String,
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub is_system: bool,
}

impl DiskStats {
    pub fn usage_percent(&self) -> Option<f64> {
        (self.total > 0).then(|| self.used as f64 / self.total as f64 * 100.0)
    }
}

pub fn is_system_mount(mount: &str) -> bool {
    mount.starts_with("/System/Volumes/") || mount.starts_with("/private/var/vm")
}

#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceStats {
    pub name: String,
    pub rx_rate: Option<f64>,
    pub tx_rate: Option<f64>,
    pub rx_total: u64,
    pub tx_total: u64,
    pub peak_rx: f64,
    pub peak_tx: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Battery {
    pub percent: f64,
    pub charging: Option<bool>,
    pub power_source: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessTraffic {
    pub pid: u32,
    pub rate: TrafficRate,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemSample {
    pub cpu_usage: Option<f64>,
    pub cores: Option<usize>,
    pub load: Option<[f64; 3]>,
    pub memory: MemoryStats,
    pub disks: Vec<DiskStats>,
    pub interfaces: Vec<InterfaceStats>,
    pub processes: Vec<ProcessInfo>,
    pub uptime: Option<Duration>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlatformSample {
    pub cpu_user: Option<f64>,
    pub cpu_system: Option<f64>,
    pub battery: Option<Battery>,
    pub temperature: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CollectorUpdate {
    System(SystemSample),
    Platform(PlatformSample),
    Traffic(Vec<ProcessTraffic>),
    Failed { source: Source, message: String },
    Stopped(Source),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemState {
    pub cpu: CpuStats,
    pub memory: MemoryStats,
    pub disks: Vec<DiskStats>,
    pub interfaces: Vec<InterfaceStats>,
    pub processes: Vec<ProcessInfo>,
    pub battery: Option<Battery>,
    pub uptime: Option<Duration>,
    issues: BTreeMap<Source, String>,
    traffic: HashMap<u32, TrafficRate>,
}

impl SystemState {
    pub fn apply(&mut self, update: CollectorUpdate) {
        match update {
            CollectorUpdate::System(sample) => {
                self.cpu.usage = sample.cpu_usage;
                self.cpu.cores = sample.cores;
                self.cpu.load = sample.load;
                self.memory = sample.memory;
                self.disks = sample.disks;
                self.interfaces = sample.interfaces;
                self.processes = sample.processes;
                self.uptime = sample.uptime;
                self.attach_traffic();
                self.issues.remove(&Source::System);
            }
            CollectorUpdate::Platform(sample) => {
                self.cpu.user = sample.cpu_user;
                self.cpu.system = sample.cpu_system;
                self.cpu.temperature = sample.temperature;
                self.battery = sample.battery;
                self.issues.remove(&Source::Platform);
            }
            CollectorUpdate::Traffic(rows) => {
                self.traffic = rows.into_iter().map(|row| (row.pid, row.rate)).collect();
                self.attach_traffic();
                self.issues.remove(&Source::Traffic);
            }
            CollectorUpdate::Failed { source, message } => {
                self.clear(source);
                self.issues.insert(source, message);
            }
            CollectorUpdate::Stopped(source) => {
                self.issues.insert(source, "data stream stopped".into());
            }
        }
    }

    pub fn has_failures(&self) -> bool {
        !self.issues.is_empty()
    }

    pub fn issue(&self, source: Source) -> Option<&str> {
        self.issues.get(&source).map(String::as_str)
    }

    pub fn primary_disk(&self) -> Option<&DiskStats> {
        self.disks
            .iter()
            .find(|disk| disk.mount == "/")
            .or_else(|| self.disks.iter().find(|disk| !disk.is_system))
    }

    pub fn visible_disks(&self) -> Vec<&DiskStats> {
        self.disks.iter().filter(|disk| !disk.is_system).collect()
    }

    fn attach_traffic(&mut self) {
        for process in &mut self.processes {
            process.traffic = self.traffic.get(&process.identity.pid).copied();
        }
    }

    fn clear(&mut self, source: Source) {
        match source {
            Source::System => {
                self.cpu.usage = None;
                self.cpu.cores = None;
                self.cpu.load = None;
                self.memory = MemoryStats::default();
                self.disks.clear();
                self.interfaces.clear();
                self.processes.clear();
                self.uptime = None;
            }
            Source::Platform => {
                self.cpu.user = None;
                self.cpu.system = None;
                self.cpu.temperature = None;
                self.battery = None;
            }
            Source::Traffic => {
                self.traffic.clear();
                self.attach_traffic();
            }
        }
    }
}
```

- [ ] **Step 5: Create `src/history.rs`**

```rust
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
            SeriesKey::NetRx(name) | SeriesKey::NetTx(name) => {
                state.interfaces.iter().any(|interface| &interface.name == name)
            }
            _ => true,
        });
    }
}
```

In `src/lib.rs` add `pub mod history;` and `pub mod state;` (keep alphabetical order).

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test state_tests`
Expected: PASS (9 tests).

- [ ] **Step 7: Run all checks and commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
git add src tests
git commit -m "feat: add typed system state and per-series history"
```

---

### Task 3: Collector'ları ve App'i typed modele geçirme

Hâlâ senkron çalışır (Task 4 thread'e taşır), ama string metrikler ve `SystemSnapshot` tamamen kalkar. Status bandı yalnızca gerçek hatada uyarı rengine döner; sistem volume'ları listelenmez.

**Files:**
- Modify: `src/collectors/system.rs`, `src/collectors/processes.rs`, `src/collectors/macos.rs`, `src/collectors/mod.rs`, `src/model.rs`, `src/state.rs` (`issues()` eklenir), `src/app.rs`, `src/ui/widgets.rs`, `src/ui/mod.rs` (testler)
- Test: `tests/collector_tests.rs` (yeniden yazılır), `tests/ui_tests.rs`, `src/app.rs` içi testler

**Interfaces:**
- Consumes: Task 2'deki `filiz::state::*`, `filiz::history::*`.
- Produces: `SystemCollector::sample(&mut self) -> SystemSample` (`processes` boş döner); `ProcessCollector::sample(&mut self) -> Vec<ProcessInfo>`; `MacOsCollector::sample(&mut self) -> PlatformSample`; `CollectorSet::{new, system_update, platform_update, collect_all}` (`collect_all` Task 4'te silinir); `App.state: SystemState`, `App.history: History`, `App::apply_update(&mut self, CollectorUpdate)`; `SystemState::issues(&self) -> impl Iterator<Item = (Source, &str)>`.
- Removed: `ResourceMetric`, `SystemSnapshot`, `CollectorData`, `CollectorResult`, `CollectorWarning`, `MacOsMetrics`, `SystemEvent`, `NetworkSummary`, `ConnectionSummary`, `network_summary`, `Collector` trait, `parse_nettop_csv`, `collect_connections`, `App::replace_snapshot`, `App.histories`, `App.snapshot`.

- [ ] **Step 1: Rewrite `tests/collector_tests.rs` (failing)**

```rust
use filiz::collectors::macos::{parse_battery, parse_cpu_usage};
use filiz::collectors::processes::ProcessCollector;
use filiz::collectors::system::{byte_rate, percentage, SystemCollector};

#[test]
fn byte_rate_uses_elapsed_seconds_between_fixed_samples() {
    assert_eq!(byte_rate(1_000, 3_500, 2.5), Some(1_000.0));
    assert_eq!(byte_rate(3_500, 4_000, 0.5), Some(1_000.0));
}

#[test]
fn byte_rate_rejects_reset_counters_and_zero_interval() {
    assert_eq!(byte_rate(3_500, 100, 1.0), None);
    assert_eq!(byte_rate(1_000, 3_500, 0.0), None);
}

#[test]
fn percentage_converts_two_fixed_samples() {
    assert_eq!(percentage(25, 100), Some(25.0));
    assert_eq!(percentage(75, 200), Some(37.5));
    assert_eq!(percentage(0, 0), None);
}

#[test]
fn macos_cpu_parser_extracts_user_and_system_percentages() {
    assert_eq!(
        parse_cpu_usage("CPU usage: 12.50% user, 4.25% sys, 83.25% idle"),
        (Some(12.5), Some(4.25))
    );
}

#[test]
fn macos_battery_parser_extracts_typed_values() {
    let sample = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1234567)\t78%; discharging; 3:20 remaining present: true";
    let battery = parse_battery(sample).expect("battery values");
    assert_eq!(battery.percent, 78.0);
    assert_eq!(battery.power_source.as_deref(), Some("Battery Power"));
    assert_eq!(battery.charging, Some(false));
}

#[test]
fn macos_battery_parser_rejects_unexposed_percentage() {
    assert!(parse_battery("Now drawing from 'AC Power'\n No batteries available").is_none());
}

#[test]
fn macos_battery_parser_keeps_percent_when_status_is_unavailable() {
    let sample = "Now drawing from 'Battery Power'\n -InternalBattery-0\t78%; unknown;";
    let battery = parse_battery(sample).expect("battery percent");
    assert_eq!(battery.percent, 78.0);
    assert_eq!(battery.charging, None);
}

#[test]
fn system_sample_reports_memory_uptime_and_rates_after_second_sample() {
    let mut collector = SystemCollector::new();
    let first = collector.sample();
    assert!(first.memory.total.is_some());
    assert!(first.uptime.is_some());
    assert_eq!(first.cpu_usage, None, "first CPU sample has no baseline");
    assert!(first.interfaces.iter().all(|i| i.rx_rate.is_none()));
    let second = collector.sample();
    assert!(second.cpu_usage.is_some());
    assert!(second.processes.is_empty());
}

#[test]
fn process_sample_preserves_start_time_and_pid_order() {
    let mut collector = ProcessCollector::new();
    let processes = collector.sample();
    assert!(processes
        .windows(2)
        .all(|pair| pair[0].identity.pid < pair[1].identity.pid));
    let current_pid = std::process::id();
    let current = processes
        .iter()
        .find(|process| process.identity.pid == current_pid)
        .expect("current process is present");
    assert!(current.identity.start_time > 0);
    assert_eq!(current.cpu_percent, None);
    let second = collector.sample();
    let current = second
        .iter()
        .find(|process| process.identity.pid == current_pid)
        .unwrap();
    assert!(current.cpu_percent.is_some());
}
```

Run: `cargo test --test collector_tests`
Expected: FAIL — `no method named sample`.

- [ ] **Step 2: Rewrite `SystemCollector` in `src/collectors/system.rs`**

Keep `percentage`, `byte_rate` and `struct NetworkState` unchanged. Delete `impl Collector for SystemCollector`, `collect_connections`, `parse_nettop_csv`, `collect_nettop_connections`, `metric`, `warning`. Replace the imports with:

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

use sysinfo::{Disks, Networks, System};

use crate::state::{is_system_mount, DiskStats, InterfaceStats, MemoryStats, SystemSample};
```

Add to `impl SystemCollector`:

```rust
    pub fn sample(&mut self) -> SystemSample {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.disks.refresh_list();
        self.networks.refresh_list();
        let now = Instant::now();
        let elapsed = self
            .previous_at
            .map(|at| now.duration_since(at).as_secs_f64());
        let has_cpus = !self.system.cpus().is_empty();
        let cpu_usage = (has_cpus && self.previous_at.is_some())
            .then(|| f64::from(self.system.global_cpu_usage()));
        let interfaces = self.sample_interfaces(elapsed);
        self.previous_at = Some(now);
        let load = System::load_average();
        SystemSample {
            cpu_usage,
            cores: has_cpus.then(|| self.system.cpus().len()),
            load: Some([load.one, load.five, load.fifteen]),
            memory: self.sample_memory(),
            disks: self.sample_disks(),
            interfaces,
            processes: Vec::new(),
            uptime: Some(Duration::from_secs(System::uptime())),
        }
    }

    fn sample_memory(&self) -> MemoryStats {
        let total = self.system.total_memory();
        if total == 0 {
            return MemoryStats::default();
        }
        let used = self.system.used_memory();
        let available = match self.system.available_memory() {
            0 => total.saturating_sub(used),
            value => value,
        };
        MemoryStats {
            total: Some(total),
            used: Some(used),
            available: Some(available),
            free: Some(self.system.free_memory()),
            swap_used: Some(self.system.used_swap()),
            swap_total: Some(self.system.total_swap()),
        }
    }

    fn sample_disks(&self) -> Vec<DiskStats> {
        self.disks
            .list()
            .iter()
            .map(|disk| {
                let mount = disk.mount_point().to_string_lossy().into_owned();
                let total = disk.total_space();
                let free = disk.available_space();
                DiskStats {
                    is_system: is_system_mount(&mount),
                    mount,
                    total,
                    used: total.saturating_sub(free),
                    free,
                }
            })
            .collect()
    }

    fn sample_interfaces(&mut self, elapsed: Option<f64>) -> Vec<InterfaceStats> {
        let mut current = HashMap::new();
        let mut names: Vec<_> = self.networks.list().keys().cloned().collect();
        names.sort();
        let mut interfaces = Vec::with_capacity(names.len());
        for name in names {
            let network = &self.networks[&name];
            let received = network.total_received();
            let transmitted = network.total_transmitted();
            let previous = self.previous_networks.get(&name);
            let rx_rate = previous
                .zip(elapsed)
                .and_then(|(state, seconds)| byte_rate(state.received, received, seconds));
            let tx_rate = previous
                .zip(elapsed)
                .and_then(|(state, seconds)| byte_rate(state.transmitted, transmitted, seconds));
            let mut state = previous.copied().unwrap_or(NetworkState {
                received,
                transmitted,
                baseline_received: received,
                baseline_transmitted: transmitted,
                peak_received: 0.0,
                peak_transmitted: 0.0,
            });
            if received < state.received {
                state.baseline_received = received;
                state.peak_received = 0.0;
            }
            if transmitted < state.transmitted {
                state.baseline_transmitted = transmitted;
                state.peak_transmitted = 0.0;
            }
            if let Some(rate) = rx_rate {
                state.peak_received = state.peak_received.max(rate);
            }
            if let Some(rate) = tx_rate {
                state.peak_transmitted = state.peak_transmitted.max(rate);
            }
            interfaces.push(InterfaceStats {
                name: name.clone(),
                rx_rate,
                tx_rate,
                rx_total: received.saturating_sub(state.baseline_received),
                tx_total: transmitted.saturating_sub(state.baseline_transmitted),
                peak_rx: state.peak_received,
                peak_tx: state.peak_transmitted,
            });
            state.received = received;
            state.transmitted = transmitted;
            current.insert(name, state);
        }
        self.previous_networks = current;
        interfaces
    }
```

- [ ] **Step 3: Rewrite `ProcessCollector` in `src/collectors/processes.rs`**

Replace the `impl Collector for ProcessCollector` block and imports:

```rust
use std::collections::HashSet;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind, Users};

use crate::model::{ProcessIdentity, ProcessInfo};
```

```rust
impl ProcessCollector {
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
```

Merge this `sample` into the existing `impl ProcessCollector` that holds `new()`.

- [ ] **Step 4: Rewrite `MacOsCollector` in `src/collectors/macos.rs`**

Keep `BatteryReading`, `parse_cpu_usage`, `parse_battery`, `MacOsCollector::new`, `Default`. Delete `collect`, `impl Collector for MacOsCollector`, `fn warn`. Imports become:

```rust
#[cfg(target_os = "macos")]
use std::process::Command;

use sysinfo::Components;

use crate::state::{Battery, PlatformSample};
```

Add:

```rust
impl MacOsCollector {
    pub fn sample(&mut self) -> PlatformSample {
        let mut sample = PlatformSample::default();
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("/usr/bin/top").args(["-l", "1", "-n", "0"]).output() {
                if output.status.success() {
                    let (user, system) = parse_cpu_usage(&String::from_utf8_lossy(&output.stdout));
                    sample.cpu_user = user;
                    sample.cpu_system = system;
                }
            }
            if let Ok(output) = Command::new("/usr/bin/pmset").args(["-g", "batt"]).output() {
                if output.status.success() {
                    sample.battery = parse_battery(&String::from_utf8_lossy(&output.stdout)).map(
                        |reading| Battery {
                            percent: reading.percent,
                            charging: reading.charging,
                            power_source: reading.power_source,
                        },
                    );
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
```

Merge into the existing `impl MacOsCollector` block.

- [ ] **Step 5: Rewrite `src/collectors/mod.rs`**

```rust
pub mod macos;
pub mod processes;
pub mod system;

use crate::state::CollectorUpdate;
use macos::MacOsCollector;
use processes::ProcessCollector;
use system::SystemCollector;

pub struct CollectorSet {
    system: SystemCollector,
    processes: ProcessCollector,
    macos: MacOsCollector,
}

impl CollectorSet {
    pub fn new() -> Self {
        Self {
            system: SystemCollector::new(),
            processes: ProcessCollector::new(),
            macos: MacOsCollector::new(),
        }
    }

    pub fn system_update(&mut self) -> CollectorUpdate {
        let mut sample = self.system.sample();
        sample.processes = self.processes.sample();
        CollectorUpdate::System(sample)
    }

    pub fn platform_update(&mut self) -> CollectorUpdate {
        CollectorUpdate::Platform(self.macos.sample())
    }

    pub fn collect_all(&mut self) -> Vec<CollectorUpdate> {
        vec![self.system_update(), self.platform_update()]
    }
}

impl Default for CollectorSet {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 6: Trim `src/model.rs`**

Delete `ResourceMetric`, `MacOsMetrics`, `SystemEvent`, `CollectorWarning`, `CollectorData`, `ConnectionSummary`, `NetworkSummary`, `CollectorResult`, `SystemSnapshot`, `network_summary`. Keep `ProcessIdentity`, `ActionKind`, `PendingAction`, `ConfirmedAction`, `CancelledAction`, `TrafficRate`, `ProcessInfo`, `SortMode`, `AppMode`, `filter_processes`, `sort_processes` and comparators. Remove the now-unused `use std::time::SystemTime;`.

Add to `impl SystemState` in `src/state.rs`:

```rust
    pub fn issues(&self) -> impl Iterator<Item = (Source, &str)> {
        self.issues
            .iter()
            .map(|(source, message)| (*source, message.as_str()))
    }
```

- [ ] **Step 7: Move `App` to `SystemState`**

In `src/app.rs`:

- Imports: remove `SystemSnapshot` from the `crate::model` list; add `use crate::history::{History, SeriesKey};` and `use crate::state::{CollectorUpdate, SystemState};`.
- `struct App`: replace `pub snapshot: Option<SystemSnapshot>` with `pub state: SystemState` and `pub histories: [Vec<u64>; 4]` with `pub history: History`. In `App::new`: `state: SystemState::default()`, `history: History::new(60)`.
- Replace `replace_snapshot` with:

```rust
    pub fn apply_update(&mut self, update: CollectorUpdate) {
        let is_system = matches!(update, CollectorUpdate::System(_));
        self.state.apply(update);
        if is_system {
            self.history.record(&self.state);
        }
        self.reconcile_selection();
    }
```

- `visible_processes`:

```rust
    pub fn visible_processes(&self) -> Vec<ProcessInfo> {
        let mut processes = filter_processes(&self.state.processes, &self.filter);
        sort_processes(&mut processes, self.sort);
        processes
    }
```

- Delete `metric_value`, `disk_usage`, `network_rate`.
- In `run`, replace every `app.replace_snapshot(collectors.snapshot());` with:

```rust
for update in collectors.collect_all() {
    app.apply_update(update);
}
```

- `SeriesKey` is used only by tests in this task; if clippy reports it unused in `app.rs`, import it inside `mod tests` instead.

Update the tests at the top of `src/app.rs`:

```rust
    fn app_with_processes() -> App {
        let mut app = App::new(Duration::from_secs(2));
        app.apply_update(CollectorUpdate::System(SystemSample {
            processes: vec![process(20, 10.0), process(10, 20.0)],
            ..Default::default()
        }));
        app
    }
```

(`use crate::state::SystemSample;` inside `mod tests`; remove `use std::time::SystemTime;`.)

Replace `replacing_snapshot_keeps_selection_by_full_identity` with:

```rust
    #[test]
    fn replacing_state_keeps_selection_by_full_identity() {
        let mut app = app_with_processes();
        app.handle_key(key(KeyCode::Down));
        app.apply_update(CollectorUpdate::System(SystemSample {
            processes: vec![process(20, 80.0), process(10, 1.0)],
            ..Default::default()
        }));
        assert_eq!(app.selected_process().unwrap().identity.pid, 20);
        let mut reused = process(20, 80.0);
        reused.identity.start_time += 1;
        app.apply_update(CollectorUpdate::System(SystemSample {
            processes: vec![reused, process(10, 1.0)],
            ..Default::default()
        }));
        assert_eq!(app.selected_process().unwrap().identity.pid, 10);
    }
```

Replace `network_history_keeps_rate_changes_above_100_kilobytes` with:

```rust
    #[test]
    fn network_history_is_recorded_per_interface() {
        let mut app = app_with_processes();
        app.apply_update(CollectorUpdate::System(SystemSample {
            interfaces: vec![crate::state::InterfaceStats {
                name: "en0".into(),
                rx_rate: Some(200_000.0),
                tx_rate: Some(1_000.0),
                rx_total: 0,
                tx_total: 0,
                peak_rx: 0.0,
                peak_tx: 0.0,
            }],
            ..Default::default()
        }));
        assert_eq!(
            app.history.series(&SeriesKey::NetRx("en0".into())).last(),
            Some(&200_000)
        );
    }
```

In `self_process_does_not_enter_action_confirmation`, replace the snapshot lines with:

```rust
        app.apply_update(CollectorUpdate::System(SystemSample {
            processes: vec![process(self_pid, 1.0)],
            ..Default::default()
        }));
```

- [ ] **Step 8: Move `widgets.rs` to `SystemState`**

Add imports: `use crate::history::SeriesKey;`, `use crate::state::{DiskStats, InterfaceStats, SystemState};`. Remove `ConnectionSummary, NetworkSummary, SystemSnapshot` from the model import.

Delete `fn value`, `fn disk_usage`, `fn disk_metric`, `fn network_rates`, `fn connection_row`. Replace these functions entirely:

```rust
fn cpu_detail(state: &SystemState) -> String {
    let cpu = &state.cpu;
    let cores = cpu
        .cores
        .map(|cores| format!("{cores} CORES"))
        .unwrap_or_else(|| "CORES N/A".into());
    let idle = cpu
        .idle()
        .map(|idle| format!("IDLE {idle:.0}%"))
        .unwrap_or_else(|| "IDLE N/A".into());
    let breakdown = match (cpu.user, cpu.system) {
        (Some(user), Some(system)) => format!("USER {user:.0}% · SYS {system:.0}%"),
        _ => "USER N/A · SYS N/A".into(),
    };
    let load = cpu
        .load
        .map(|load| format!("LOAD {:.2}", load[0]))
        .unwrap_or_else(|| "LOAD N/A".into());
    let temperature = cpu
        .temperature
        .map(|t| format!("TEMP {t:.0}°C"))
        .unwrap_or_else(|| "TEMP N/A".into());
    format!("{cores} · {idle} · {breakdown} · {load} · {temperature}")
}

fn opt_bytes(value: Option<u64>) -> String {
    value.map(bytes).unwrap_or_else(|| "N/A".into())
}

fn memory_detail(state: &SystemState) -> String {
    let memory = &state.memory;
    format!(
        "USED {} · FREE {} · AVAIL {} · SWAP {}",
        opt_bytes(memory.used),
        opt_bytes(memory.free),
        opt_bytes(memory.available),
        opt_bytes(memory.swap_used)
    )
}

fn disk_detail(state: &SystemState) -> String {
    let disk = state.primary_disk();
    format!(
        "USED {} · FREE {} · TOTAL {}",
        opt_bytes(disk.map(|d| d.used)),
        opt_bytes(disk.map(|d| d.free)),
        opt_bytes(disk.map(|d| d.total))
    )
}

fn total_rates(state: &SystemState) -> (Option<f64>, Option<f64>) {
    let sum = |pick: fn(&InterfaceStats) -> Option<f64>| {
        let values: Vec<f64> = state.interfaces.iter().filter_map(pick).collect();
        (!values.is_empty()).then(|| values.iter().sum())
    };
    (sum(|i| i.rx_rate), sum(|i| i.tx_rate))
}

fn selected_interface(app: &App) -> Option<&InterfaceStats> {
    let interfaces = &app.state.interfaces;
    interfaces.get(app.ui.network_interface.min(interfaces.len().saturating_sub(1)))
}

fn network_row(interface: &InterfaceStats) -> Row<'static> {
    Row::new([
        interface.name.clone(),
        interface.rx_rate.map(rate).unwrap_or_else(|| "N/A".into()),
        interface.tx_rate.map(rate).unwrap_or_else(|| "N/A".into()),
        bytes(interface.rx_total.saturating_add(interface.tx_total)),
        format!("↓ {} ↑ {}", rate(interface.peak_rx), rate(interface.peak_tx)),
    ])
}

fn traffic_row(process: &ProcessInfo) -> Row<'static> {
    let traffic = process.traffic;
    Row::new([
        process.name.clone(),
        process.identity.pid.to_string(),
        traffic.map(|t| rate(t.rx)).unwrap_or_else(|| "N/A".into()),
        traffic.map(|t| rate(t.tx)).unwrap_or_else(|| "N/A".into()),
    ])
}

fn disk_row(disk: &DiskStats) -> Row<'static> {
    Row::new([
        disk.mount.clone(),
        percent(disk.usage_percent()),
        bytes(disk.used),
        bytes(disk.free),
        bytes(disk.total),
    ])
}
```

Apply these edits inside the public functions:

| Function | Old | New |
|---|---|---|
| `status` | `warnings == 0` checks | `let failing = app.state.has_failures();` and use `!failing` |
| `status` | uptime `value(snapshot, "uptime")…` | `app.state.uptime.map(\|d\| format_uptime(d.as_secs())).unwrap_or_else(\|\| "N/A".into())` |
| `status` | battery | `app.state.battery.as_ref().map(\|b\| format!("{:.0}%", b.percent)).unwrap_or_else(\|\| "N/A".into())` |
| `status`, `more` | temperature | `app.state.cpu.temperature.map(\|t\| format!("{t:.0}°C")).unwrap_or_else(\|\| "N/A".into())` |
| `resources` | `cpu`, `memory`, `disk` | `app.state.cpu.usage`, `app.state.memory.usage_percent()`, `app.state.primary_disk().and_then(DiskStats::usage_percent)` |
| `resources` | `(rx, tx)` | `total_rates(&app.state)` |
| `resources` | details | `cpu_detail(&app.state)`, `memory_detail(&app.state)`, `disk_detail(&app.state)` |
| `resources` | `&app.histories[0..=3]` | `app.history.series(&SeriesKey::Cpu)`, `…Memory`, `…Disk`, and for NETWORK `selected_interface(app).map(\|i\| app.history.series(&SeriesKey::NetRx(i.name.clone()))).unwrap_or_default()`; change the card tuple's last element type from `&Vec<u64>` to `Vec<u64>` and `resource_card`'s parameter to `&(&str, String, String, Option<f64>, Vec<u64>)`, using `.data(history)` with `history: &Vec<u64>` |
| `network` | `summaries`/`selected` | `let selected = selected_interface(app);` |
| `network` | download/upload cards | main text `rate_or_na(selected.and_then(\|i\| i.rx_rate))` / `…tx_rate`; histories `SeriesKey::NetRx(name)` / `SeriesKey::NetTx(name)` (empty `Vec` when `selected` is `None`); upload color `palette.chart_tx`, download `palette.chart_rx`; `network_card` takes `history: &[u64]` |
| `network` | interfaces table rows | `app.state.interfaces.iter().map(network_row)` |
| `network` | connections table | see block below |
| `disks` | row building | `app.state.visible_disks().into_iter().map(disk_row).collect::<Vec<_>>()`; header `["MOUNT", "USAGE", "USED", "FREE", "TOTAL"]`; 5 widths `[Length(18), Length(10), Length(14), Length(14), Length(14)]`; empty row has 5 cells |
| `details` | `warning` | `let warning = app.state.issues().next();` and render `format!(" WARNING  {source:?}: {message}")` |
| `confirmation_modal` | name lookup | `app.state.processes.iter().find(\|p\| p.identity == pending.identity())` |

Network traffic table (replaces the connections table and its offset code):

```rust
    let mut talkers: Vec<&ProcessInfo> = app
        .state
        .processes
        .iter()
        .filter(|process| process.traffic.is_some())
        .collect();
    talkers.sort_by(|a, b| {
        let total = |p: &ProcessInfo| p.traffic.map_or(0.0, |t| t.rx + t.tx);
        total(b).total_cmp(&total(a))
    });
    let offset = app
        .ui
        .scroll_offsets
        .get(&Panel::Network)
        .copied()
        .unwrap_or(0) as usize;
    let empty = if app.state.issue(crate::state::Source::Traffic).is_some() {
        "Traffic unavailable"
    } else {
        "No process traffic yet"
    };
    let rows: Vec<Row> = talkers
        .iter()
        .skip(offset.min(talkers.len()))
        .map(|process| traffic_row(process))
        .collect();
    let traffic_table = Table::new(
        if rows.is_empty() {
            vec![Row::new([empty, "", "", ""])]
        } else {
            rows
        },
        [
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(["PROCESS", "PID", "DOWN", "UP"]).style(
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" PROCESS TRAFFIC ({}) ", talkers.len()))
            .title_style(Style::default().fg(palette.accent))
            .border_style(Style::default().fg(palette.border))
            .style(Style::default().bg(palette.surface)),
    )
    .style(Style::default().fg(palette.text));
    frame.render_widget(traffic_table, sections[2]);
```

- [ ] **Step 9: Update UI test fixtures**

In `tests/ui_tests.rs` and in `mod tests` of `src/ui/mod.rs`, replace the `sample_app` body with:

```rust
fn sample_app() -> App {
    let mut app = App::new(Duration::from_secs(2));
    app.apply_update(CollectorUpdate::System(SystemSample {
        cpu_usage: Some(35.0),
        memory: MemoryStats {
            total: Some(32 * 1024 * 1024 * 1024),
            used: Some(20 * 1024 * 1024 * 1024),
            ..Default::default()
        },
        disks: vec![DiskStats {
            mount: "/".into(),
            total: 100,
            used: 91,
            free: 9,
            is_system: false,
        }],
        interfaces: vec![InterfaceStats {
            name: "en0".into(),
            rx_rate: Some(1024.0),
            tx_rate: Some(512.0),
            rx_total: 0,
            tx_total: 0,
            peak_rx: 0.0,
            peak_tx: 0.0,
        }],
        processes: vec![ProcessInfo {
            identity: ProcessIdentity {
                pid: 42,
                start_time: 100,
            },
            name: "example-worker".into(),
            command: "/usr/bin/example-worker".into(),
            cpu_percent: Some(12.5),
            memory_bytes: Some(1024 * 1024),
            user: Some("user".into()),
            status: Some("Running".into()),
            traffic: None,
        }],
        uptime: Some(Duration::from_secs(3600)),
        ..Default::default()
    }));
    app
}
```

Imports: `use filiz::state::{CollectorUpdate, DiskStats, InterfaceStats, MemoryStats, SystemSample};` (in `src/ui/mod.rs` use `crate::state::…`); drop `ResourceMetric`, `SystemSnapshot`, `SystemTime`, and the `metric` helper.

Add to `tests/ui_tests.rs`:

```rust
#[test]
fn missing_sensors_keep_status_normal_but_failures_warn() {
    let mut app = sample_app();
    assert!(screen(&app, 110, 35).contains("SYSTEM NORMAL"));
    app.apply_update(CollectorUpdate::Failed {
        source: filiz::state::Source::Platform,
        message: "top failed".into(),
    });
    assert!(screen(&app, 110, 35).contains("CHECK METRICS"));
}

#[test]
fn disks_workspace_hides_system_volumes() {
    let mut app = sample_app();
    let mut sample = SystemSample::default();
    sample.disks = vec![
        DiskStats { mount: "/".into(), total: 100, used: 50, free: 50, is_system: false },
        DiskStats { mount: "/System/Volumes/VM".into(), total: 100, used: 1, free: 99, is_system: true },
    ];
    app.apply_update(CollectorUpdate::System(sample));
    app.ui.workspace = Workspace::Disks;
    let output = screen(&app, 110, 35);
    assert!(!output.contains("/System/Volumes/VM"));
}
```

- [ ] **Step 10: Run all checks and commit**

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: PASS. Then `cargo run --release` manually: dashboard shows values; `Q` quits.

```bash
git add src tests
git commit -m "refactor: drive collectors and UI from typed system state"
```

---

### Task 4: Collector runtime ve bloklamayan ana döngü

UI thread'inde hiçbir collector çalışmaz; arayüz donması (hata 12) biter.

**Files:**
- Create: `src/collectors/runtime.rs`
- Modify: `src/collectors/mod.rs` (`CollectorSet` silinir, `pub mod runtime;`), `src/app.rs` (`pump`, `run`), `src/main.rs`
- Test: `tests/runtime_tests.rs`

**Interfaces:**
- Consumes: `CollectorUpdate`, `Source`, `SystemCollector::sample`, `ProcessCollector::sample`, `MacOsCollector::sample`.
- Produces (`filiz::collectors::runtime`): `Control::{Refresh, Stop}`; `Job = Box<dyn FnMut() -> CollectorUpdate + Send>`; `WorkerSpec { source: Source, interval: Duration, job: Job }`; `CollectorRuntime::{start(Sender<CollectorUpdate>) -> Self, with_workers(Vec<WorkerSpec>, Sender<CollectorUpdate>) -> Self, refresh(&self)}` and `Drop` (Stop + join). Task 8 adds `CollectorRuntime::add_child_killer`.
- Produces (`filiz::app`): `App::pump(&mut self, rx: &Receiver<CollectorUpdate>) -> bool`; `run(terminal, app, runtime: CollectorRuntime, rx: Receiver<CollectorUpdate>) -> Result<()>`.

- [ ] **Step 1: Write the failing tests**

`tests/runtime_tests.rs`:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use filiz::app::App;
use filiz::collectors::runtime::{CollectorRuntime, WorkerSpec};
use filiz::state::{CollectorUpdate, PlatformSample, Source, SystemSample};

fn counting_worker(interval: Duration, delay: Duration, counter: Arc<AtomicUsize>) -> WorkerSpec {
    WorkerSpec {
        source: Source::Platform,
        interval,
        job: Box::new(move || {
            std::thread::sleep(delay);
            counter.fetch_add(1, Ordering::SeqCst);
            CollectorUpdate::Platform(PlatformSample::default())
        }),
    }
}

#[test]
fn worker_runs_immediately_and_then_on_interval() {
    let (tx, rx) = mpsc::channel();
    let counter = Arc::new(AtomicUsize::new(0));
    let runtime = CollectorRuntime::with_workers(
        vec![counting_worker(Duration::from_millis(50), Duration::ZERO, counter.clone())],
        tx,
    );
    let first = rx.recv_timeout(Duration::from_millis(500)).unwrap();
    assert!(matches!(first, CollectorUpdate::Platform(_)));
    std::thread::sleep(Duration::from_millis(180));
    drop(runtime);
    assert!(counter.load(Ordering::SeqCst) >= 3);
}

#[test]
fn refresh_triggers_collection_before_interval() {
    let (tx, rx) = mpsc::channel();
    let counter = Arc::new(AtomicUsize::new(0));
    let runtime = CollectorRuntime::with_workers(
        vec![counting_worker(Duration::from_secs(60), Duration::ZERO, counter.clone())],
        tx,
    );
    rx.recv_timeout(Duration::from_millis(500)).unwrap();
    runtime.refresh();
    rx.recv_timeout(Duration::from_millis(500)).unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[test]
fn pump_never_blocks_on_slow_worker() {
    let (tx, rx) = mpsc::channel();
    let _runtime = CollectorRuntime::with_workers(
        vec![counting_worker(
            Duration::from_secs(60),
            Duration::from_secs(1),
            Arc::new(AtomicUsize::new(0)),
        )],
        tx,
    );
    let mut app = App::new(Duration::from_secs(2));
    let started = Instant::now();
    assert!(!app.pump(&rx));
    assert!(started.elapsed() < Duration::from_millis(20));
}

#[test]
fn pump_applies_every_queued_update() {
    let (tx, rx) = mpsc::channel();
    tx.send(CollectorUpdate::System(SystemSample {
        cpu_usage: Some(10.0),
        ..Default::default()
    }))
    .unwrap();
    tx.send(CollectorUpdate::System(SystemSample {
        cpu_usage: Some(20.0),
        ..Default::default()
    }))
    .unwrap();
    let mut app = App::new(Duration::from_secs(2));
    assert!(app.pump(&rx));
    assert_eq!(app.state.cpu.usage, Some(20.0));
}

#[test]
fn runtime_drop_joins_workers() {
    let (tx, rx) = mpsc::channel();
    let runtime = CollectorRuntime::with_workers(
        vec![counting_worker(
            Duration::from_secs(60),
            Duration::ZERO,
            Arc::new(AtomicUsize::new(0)),
        )],
        tx,
    );
    rx.recv_timeout(Duration::from_millis(500)).unwrap();
    let started = Instant::now();
    drop(runtime);
    assert!(started.elapsed() < Duration::from_millis(500));
    assert!(matches!(
        rx.recv_timeout(Duration::from_millis(100)),
        Err(mpsc::RecvTimeoutError::Disconnected)
    ));
}

#[test]
fn panicking_worker_reports_stopped_source() {
    let (tx, rx) = mpsc::channel();
    let _runtime = CollectorRuntime::with_workers(
        vec![WorkerSpec {
            source: Source::Traffic,
            interval: Duration::from_secs(60),
            job: Box::new(|| panic!("collector bug")),
        }],
        tx,
    );
    let update = rx.recv_timeout(Duration::from_millis(500)).unwrap();
    assert_eq!(update, CollectorUpdate::Stopped(Source::Traffic));
}
```

Run: `cargo test --test runtime_tests`
Expected: FAIL — `could not find runtime in collectors`.

- [ ] **Step 2: Create `src/collectors/runtime.rs`**

```rust
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
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
struct StopGuard {
    source: Source,
    updates: Sender<CollectorUpdate>,
    armed: bool,
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
        Self::with_workers(
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
            updates,
        )
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
    let mut guard = StopGuard {
        source: worker.source,
        updates: updates.clone(),
        armed: true,
    };
    loop {
        if updates.send((worker.job)()).is_err() {
            break;
        }
        match control.recv_timeout(worker.interval) {
            Ok(Control::Refresh) | Err(RecvTimeoutError::Timeout) => continue,
            Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    guard.armed = false;
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
```

Note: a panicking job unwinds through `run_worker`, dropping `guard` while `armed` is still `true`, which sends `Stopped`.

- [ ] **Step 3: Remove `CollectorSet`**

In `src/collectors/mod.rs` keep only:

```rust
pub mod macos;
pub mod processes;
pub mod runtime;
pub mod system;
```

- [ ] **Step 4: Add `App::pump` and rewrite `run` in `src/app.rs`**

```rust
    /// Apply every queued collector update without blocking. Returns true if anything changed.
    pub fn pump(&mut self, updates: &Receiver<CollectorUpdate>) -> bool {
        let mut changed = false;
        while let Ok(update) = updates.try_recv() {
            self.apply_update(update);
            changed = true;
        }
        changed
    }
```

Replace `run`:

```rust
pub fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    runtime: CollectorRuntime,
    updates: Receiver<CollectorUpdate>,
) -> Result<()> {
    const INPUT_WAIT: Duration = Duration::from_millis(50);
    let mut dirty = true;
    loop {
        if app.pump(&updates) {
            dirty = true;
        }
        if app.clear_expired_notice() {
            dirty = true;
        }
        if dirty {
            terminal.draw(|frame| ui::render(frame, app))?;
            dirty = false;
        }
        if !event::poll(INPUT_WAIT)? {
            continue;
        }
        match event::read()? {
            Event::Key(key) => {
                match app.handle_key(key) {
                    AppCommand::Quit => break,
                    AppCommand::Refresh => runtime.refresh(),
                    AppCommand::ConfirmAction(action) => {
                        let pid = action.identity().pid;
                        match ProcessAction::execute(action) {
                            Ok(()) => app.show_notice(format!("Signal sent to PID {pid}.")),
                            Err(error) => app.show_notice(error.to_user_message()),
                        }
                        runtime.refresh();
                    }
                    _ => {}
                }
                dirty = true;
            }
            Event::Mouse(mouse) => {
                app.handle_mouse(mouse);
                dirty = true;
            }
            Event::Resize(_, _) => dirty = true,
            _ => {}
        }
    }
    drop(runtime);
    Ok(())
}
```

Imports: `use std::sync::mpsc::Receiver;`, `use crate::collectors::runtime::CollectorRuntime;`; remove `use crate::collectors::CollectorSet;` and the now-unused `Instant` if clippy says so (`notice_until` still uses it — keep). Remove the `refresh: Duration` field only if clippy flags it; otherwise leave it (Task 5 removes it).

- [ ] **Step 5: Start the runtime in `src/main.rs` before the splash**

Replace the body of `main` after `let _guard = TerminalGuard::enter()?;`:

```rust
    let (updates_tx, updates_rx) = std::sync::mpsc::channel();
    let runtime = filiz::collectors::runtime::CollectorRuntime::start(updates_tx);
    print!(
        "{}\n  for betül, with love ♡\n",
        include_str!("../assets/logo.ansi")
    );
    stdout().flush()?;
    std::thread::sleep(Duration::from_millis(650));
    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = app::App::new(Duration::from_secs(2));
    app::run(&mut terminal, &mut app, runtime, updates_rx)
```

(Splash rendering itself is fixed in Task 9.)

- [ ] **Step 6: Run tests, checks, manual smoke, commit**

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: PASS (6 new runtime tests).

Manual: `cargo run --release`, hold `↓` in the process list for 5 s — selection moves continuously without half-second stalls; `Q` exits in under 1 s.

```bash
git add src tests
git commit -m "feat: collect metrics on background threads"
```

---

### Task 5: Girdi katmanı — Action, keymap ve ListState

Saf modüller; henüz App'e bağlanmaz (Task 6 bağlar).

**Files:**
- Create: `src/input/mod.rs`, `src/input/action.rs`, `src/input/keymap.rs`, `src/input/list.rs`
- Modify: `src/lib.rs` (`pub mod input;`), `src/ui/state.rs` (`PanelId`, `Workspace::panels` eklenir)
- Test: `tests/keymap_tests.rs`

**Interfaces:**
- Produces (`filiz::ui::state`): `PanelId::{Resources, Processes, Details, Interfaces, Traffic, Disks, Settings}` (`Clone, Copy, Debug, PartialEq, Eq, Hash`); `Workspace::panels(self) -> &'static [PanelId]` — Overview `[Resources, Processes, Details]`, Processes `[Processes, Details]`, Network `[Interfaces, Traffic]`, Disks `[Disks]`, More `[Settings]`.
- Produces (`filiz::input::action`): `Action` (variants below), `Effect::{Quit, Refresh, SendSignal(ConfirmedAction)}`.
- Produces (`filiz::input::keymap`): `Context::{Confirm, Detail, Filter, Menu, Panel(PanelId), Workspace(Workspace), Global}`, `Context::exclusive(self) -> bool`, `KeySpec { code: KeyCode, shift: bool }` with `from_event(&KeyEvent)`, `Binding { context, key, action, label, hint }`, `static BINDINGS: &[Binding]`, `resolve(stack: &[Context], event: &KeyEvent) -> Option<Action>`, `hints(stack: &[Context]) -> Vec<&'static Binding>`.
- Produces (`filiz::input::list`): `ListState { selected: usize, offset: usize }` with `clamp(&mut self, len: usize, viewport: usize)` and `move_by(&mut self, delta: isize, len: usize, viewport: usize)`.

- [ ] **Step 1: Write the failing tests**

`tests/keymap_tests.rs`:

```rust
use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use filiz::input::action::Action;
use filiz::input::keymap::{hints, resolve, Context, BINDINGS};
use filiz::input::list::ListState;
use filiz::ui::state::{PanelId, Workspace};

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn shifted(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
}

fn dashboard(panel: PanelId, workspace: Workspace) -> Vec<Context> {
    vec![
        Context::Panel(panel),
        Context::Workspace(workspace),
        Context::Global,
    ]
}

#[test]
fn no_context_binds_the_same_key_twice() {
    let mut seen = HashSet::new();
    for binding in BINDINGS {
        assert!(
            seen.insert((format!("{:?}", binding.context), format!("{:?}", binding.key))),
            "duplicate binding {:?} in {:?}",
            binding.key,
            binding.context
        );
    }
}

#[test]
fn m_opens_menu_and_s_cycles_sort() {
    let stack = dashboard(PanelId::Processes, Workspace::Processes);
    assert_eq!(resolve(&stack, &press(KeyCode::Char('m'))), Some(Action::ToggleMenu));
    assert_eq!(resolve(&stack, &press(KeyCode::Char('s'))), Some(Action::CycleSort));
    assert_eq!(resolve(&stack, &press(KeyCode::Char('c'))), None);
}

#[test]
fn process_keys_only_work_when_process_panel_is_focused() {
    let processes = dashboard(PanelId::Processes, Workspace::Overview);
    let network = dashboard(PanelId::Interfaces, Workspace::Network);
    assert_eq!(resolve(&processes, &press(KeyCode::Char('k'))), Some(Action::Terminate));
    assert_eq!(resolve(&processes, &shifted('K')), Some(Action::Kill));
    assert_eq!(resolve(&network, &press(KeyCode::Char('k'))), None);
    assert_eq!(resolve(&network, &press(KeyCode::Char('f'))), None);
}

#[test]
fn shift_falls_back_to_unshifted_binding() {
    let stack = dashboard(PanelId::Resources, Workspace::Overview);
    assert_eq!(resolve(&stack, &shifted('Q')), Some(Action::Quit));
    assert_eq!(
        resolve(&stack, &KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Some(Action::Quit)
    );
}

#[test]
fn confirm_context_swallows_unbound_keys() {
    let stack = vec![Context::Confirm];
    assert_eq!(resolve(&stack, &press(KeyCode::Char('q'))), None);
    assert_eq!(resolve(&stack, &press(KeyCode::Enter)), None);
    assert_eq!(resolve(&stack, &press(KeyCode::Char('y'))), Some(Action::Confirm));
    assert_eq!(resolve(&stack, &press(KeyCode::Esc)), Some(Action::Cancel));
}

#[test]
fn filter_accepts_uppercase_and_turkish_characters() {
    let stack = vec![Context::Filter];
    assert_eq!(resolve(&stack, &shifted('Q')), Some(Action::FilterInput('Q')));
    assert_eq!(
        resolve(&stack, &press(KeyCode::Char('İ'))),
        Some(Action::FilterInput('İ'))
    );
    assert_eq!(
        resolve(&stack, &press(KeyCode::Char('ş'))),
        Some(Action::FilterInput('ş'))
    );
    assert_eq!(resolve(&stack, &press(KeyCode::Enter)), Some(Action::FilterSubmit));
    assert_eq!(resolve(&stack, &press(KeyCode::Backspace)), Some(Action::FilterBackspace));
    assert_eq!(resolve(&stack, &press(KeyCode::Tab)), None);
}

#[test]
fn key_release_events_are_ignored() {
    let mut event = press(KeyCode::Char('q'));
    event.kind = KeyEventKind::Release;
    assert_eq!(resolve(&[Context::Global], &event), None);
}

#[test]
fn hints_follow_the_active_context() {
    let labels = |stack: &[Context]| {
        hints(stack)
            .iter()
            .filter_map(|binding| binding.hint)
            .collect::<Vec<_>>()
    };
    assert_eq!(labels(&[Context::Confirm]), vec!["CONFIRM", "CANCEL"]);
    let processes = labels(&dashboard(PanelId::Processes, Workspace::Processes));
    assert!(processes.contains(&"FILTER"));
    assert!(processes.contains(&"QUIT"));
    let network = labels(&dashboard(PanelId::Interfaces, Workspace::Network));
    assert!(!network.contains(&"FILTER"));
    assert!(network.contains(&"PANEL"));
}

#[test]
fn list_state_moves_within_bounds_and_keeps_selection_visible() {
    let mut list = ListState::default();
    list.move_by(7, 10, 4);
    assert_eq!(list, ListState { selected: 7, offset: 4 });
    list.move_by(100, 10, 4);
    assert_eq!(list, ListState { selected: 9, offset: 6 });
    list.move_by(-100, 10, 4);
    assert_eq!(list, ListState { selected: 0, offset: 0 });
}

#[test]
fn list_state_survives_shrinking_list_and_viewport() {
    let mut list = ListState { selected: 40, offset: 35 };
    list.clamp(5, 1);
    assert_eq!(list, ListState { selected: 4, offset: 4 });
    list.clamp(0, 0);
    assert_eq!(list, ListState::default());
    list.move_by(3, 0, 10);
    assert_eq!(list, ListState::default());
}

#[test]
fn workspaces_declare_their_panels() {
    assert_eq!(
        Workspace::Overview.panels(),
        &[PanelId::Resources, PanelId::Processes, PanelId::Details]
    );
    assert_eq!(Workspace::Processes.panels(), &[PanelId::Processes, PanelId::Details]);
    assert_eq!(Workspace::Network.panels(), &[PanelId::Interfaces, PanelId::Traffic]);
}
```

Run: `cargo test --test keymap_tests`
Expected: FAIL — `could not find input in filiz`.

- [ ] **Step 2: Add `PanelId` and `Workspace::panels` to `src/ui/state.rs`**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelId {
    Resources,
    Processes,
    Details,
    Interfaces,
    Traffic,
    Disks,
    Settings,
}

impl PanelId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Resources => "Resources",
            Self::Processes => "Processes",
            Self::Details => "Details",
            Self::Interfaces => "Interfaces",
            Self::Traffic => "Traffic",
            Self::Disks => "Disks",
            Self::Settings => "Settings",
        }
    }
}
```

Inside `impl Workspace`:

```rust
    pub fn panels(self) -> &'static [PanelId] {
        match self {
            Self::Overview => &[PanelId::Resources, PanelId::Processes, PanelId::Details],
            Self::Processes => &[PanelId::Processes, PanelId::Details],
            Self::Network => &[PanelId::Interfaces, PanelId::Traffic],
            Self::Disks => &[PanelId::Disks],
            Self::More => &[PanelId::Settings],
        }
    }
```

- [ ] **Step 3: Create `src/input/mod.rs` and `src/input/action.rs`**

`src/input/mod.rs`:

```rust
pub mod action;
pub mod keymap;
pub mod list;
```

`src/input/action.rs`:

```rust
use crate::model::ConfirmedAction;
use crate::ui::state::{PanelId, Workspace};

/// A semantic user intent, independent of the key or mouse gesture that produced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Quit,
    Refresh,
    GoWorkspace(Workspace),
    NextWorkspace,
    PrevWorkspace,
    FocusNext,
    FocusPrev,
    Focus(PanelId),
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Home,
    End,
    Scroll(PanelId, i16),
    Select(PanelId, usize),
    Open,
    Back,
    StartFilter,
    FilterInput(char),
    FilterBackspace,
    FilterSubmit,
    CycleSort,
    Terminate,
    Kill,
    Confirm,
    Cancel,
    ToggleMenu,
    CycleTheme,
    CycleDensity,
    TogglePanel,
}

/// Work the event loop performs outside the pure `App::update`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Quit,
    Refresh,
    SendSignal(ConfirmedAction),
}
```

- [ ] **Step 4: Create `src/input/list.rs`**

```rust
/// Selection and scroll position of a list, always kept within the list bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListState {
    pub selected: usize,
    pub offset: usize,
}

impl ListState {
    pub fn clamp(&mut self, len: usize, viewport: usize) {
        if len == 0 {
            *self = Self::default();
            return;
        }
        let viewport = viewport.max(1);
        self.selected = self.selected.min(len - 1);
        if self.selected < self.offset {
            self.offset = self.selected;
        }
        if self.selected >= self.offset + viewport {
            self.offset = self.selected + 1 - viewport;
        }
        self.offset = self.offset.min(len.saturating_sub(viewport));
    }

    pub fn move_by(&mut self, delta: isize, len: usize, viewport: usize) {
        if len > 0 {
            let target = (self.selected as isize).saturating_add(delta);
            self.selected = target.clamp(0, len as isize - 1) as usize;
        }
        self.clamp(len, viewport);
    }
}
```

- [ ] **Step 5: Create `src/input/keymap.rs`**

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::action::Action;
use crate::ui::state::{PanelId, Workspace};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    Confirm,
    Detail,
    Filter,
    Menu,
    Panel(PanelId),
    Workspace(Workspace),
    Global,
}

impl Context {
    /// Exclusive contexts swallow keys they do not bind.
    pub fn exclusive(self) -> bool {
        matches!(self, Self::Confirm | Self::Filter)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeySpec {
    pub code: KeyCode,
    pub shift: bool,
}

impl KeySpec {
    const fn key(code: KeyCode) -> Self {
        Self { code, shift: false }
    }

    const fn ch(c: char) -> Self {
        Self::key(KeyCode::Char(c))
    }

    const fn shift_ch(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            shift: true,
        }
    }

    /// Letters are normalized to lowercase with `shift` carrying the case.
    pub fn from_event(event: &KeyEvent) -> Self {
        let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        match event.code {
            KeyCode::Char(c) if c.is_ascii_alphabetic() => Self {
                code: KeyCode::Char(c.to_ascii_lowercase()),
                shift: shift || c.is_ascii_uppercase(),
            },
            code => Self { code, shift },
        }
    }
}

#[derive(Debug)]
pub struct Binding {
    pub context: Context,
    pub key: KeySpec,
    pub action: Action,
    pub label: &'static str,
    pub hint: Option<&'static str>,
}

const fn bind(
    context: Context,
    key: KeySpec,
    action: Action,
    label: &'static str,
    hint: Option<&'static str>,
) -> Binding {
    Binding {
        context,
        key,
        action,
        label,
        hint,
    }
}

use Context::{Confirm, Detail, Filter, Global, Menu};
const PROCESSES: Context = Context::Panel(PanelId::Processes);

pub static BINDINGS: &[Binding] = &[
    bind(Confirm, KeySpec::ch('y'), Action::Confirm, "Y", Some("CONFIRM")),
    bind(Confirm, KeySpec::ch('n'), Action::Cancel, "N", Some("CANCEL")),
    bind(Confirm, KeySpec::key(KeyCode::Esc), Action::Cancel, "ESC", None),
    bind(Filter, KeySpec::key(KeyCode::Enter), Action::FilterSubmit, "ENTER", Some("DONE")),
    bind(Filter, KeySpec::key(KeyCode::Esc), Action::FilterSubmit, "ESC", Some("CLOSE")),
    bind(Filter, KeySpec::key(KeyCode::Backspace), Action::FilterBackspace, "⌫", None),
    bind(Detail, KeySpec::key(KeyCode::Esc), Action::Back, "ESC", Some("CLOSE")),
    bind(Detail, KeySpec::ch('k'), Action::Terminate, "K", Some("TERMINATE")),
    bind(Detail, KeySpec::shift_ch('k'), Action::Kill, "⇧K", Some("KILL")),
    bind(Menu, KeySpec::key(KeyCode::Esc), Action::ToggleMenu, "ESC", Some("CLOSE MENU")),
    bind(PROCESSES, KeySpec::key(KeyCode::Enter), Action::Open, "ENTER", Some("DETAIL")),
    bind(PROCESSES, KeySpec::ch('f'), Action::StartFilter, "F", Some("FILTER")),
    bind(PROCESSES, KeySpec::ch('s'), Action::CycleSort, "S", Some("SORT")),
    bind(PROCESSES, KeySpec::ch('k'), Action::Terminate, "K", Some("TERMINATE")),
    bind(PROCESSES, KeySpec::shift_ch('k'), Action::Kill, "⇧K", Some("KILL")),
    bind(Global, KeySpec::ch('q'), Action::Quit, "Q", Some("QUIT")),
    bind(Global, KeySpec::key(KeyCode::Tab), Action::FocusNext, "TAB", Some("PANEL")),
    bind(Global, KeySpec::key(KeyCode::BackTab), Action::FocusPrev, "⇧TAB", None),
    bind(Global, KeySpec::key(KeyCode::Up), Action::MoveUp, "↑↓", Some("MOVE")),
    bind(Global, KeySpec::key(KeyCode::Down), Action::MoveDown, "↓", None),
    bind(Global, KeySpec::key(KeyCode::PageUp), Action::PageUp, "PGUP", None),
    bind(Global, KeySpec::key(KeyCode::PageDown), Action::PageDown, "PGDN", None),
    bind(Global, KeySpec::key(KeyCode::Home), Action::Home, "HOME", None),
    bind(Global, KeySpec::key(KeyCode::End), Action::End, "END", None),
    bind(Global, KeySpec::key(KeyCode::Left), Action::PrevWorkspace, "←→", Some("WORKSPACE")),
    bind(Global, KeySpec::key(KeyCode::Right), Action::NextWorkspace, "→", None),
    bind(Global, KeySpec::ch('1'), Action::GoWorkspace(Workspace::Overview), "1", None),
    bind(Global, KeySpec::ch('2'), Action::GoWorkspace(Workspace::Processes), "2", None),
    bind(Global, KeySpec::ch('3'), Action::GoWorkspace(Workspace::Network), "3", None),
    bind(Global, KeySpec::ch('4'), Action::GoWorkspace(Workspace::Disks), "4", None),
    bind(Global, KeySpec::ch('5'), Action::GoWorkspace(Workspace::More), "5", None),
    bind(Global, KeySpec::key(KeyCode::Esc), Action::Back, "ESC", None),
    bind(Global, KeySpec::ch('m'), Action::ToggleMenu, "M", Some("MENU")),
    bind(Global, KeySpec::ch('h'), Action::TogglePanel, "H", Some("HIDE")),
    bind(Global, KeySpec::ch('l'), Action::CycleDensity, "L", Some("LAYOUT")),
    bind(Global, KeySpec::ch('t'), Action::CycleTheme, "T", Some("THEME")),
    bind(Global, KeySpec::ch('r'), Action::Refresh, "R", Some("REFRESH")),
];

fn lookup(context: Context, key: KeySpec) -> Option<&'static Binding> {
    BINDINGS
        .iter()
        .find(|binding| binding.context == context && binding.key == key)
}

pub fn resolve(stack: &[Context], event: &KeyEvent) -> Option<Action> {
    if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }
    let control = event.modifiers.contains(KeyModifiers::CONTROL);
    if control && event.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }
    let key = KeySpec::from_event(event);
    for &context in stack {
        if context == Context::Filter {
            if let KeyCode::Char(c) = event.code {
                if !control && !c.is_control() {
                    return Some(Action::FilterInput(c));
                }
            }
        }
        let unshifted = KeySpec { shift: false, ..key };
        let found = lookup(context, key).or_else(|| key.shift.then(|| lookup(context, unshifted)).flatten());
        if let Some(binding) = found {
            return Some(binding.action);
        }
        if context.exclusive() {
            return None;
        }
    }
    None
}

/// Hinted bindings reachable from `stack`, in priority order, without shadowed keys.
pub fn hints(stack: &[Context]) -> Vec<&'static Binding> {
    let mut taken: Vec<KeySpec> = Vec::new();
    let mut result = Vec::new();
    for &context in stack {
        for binding in BINDINGS.iter().filter(|binding| binding.context == context) {
            if taken.contains(&binding.key) {
                continue;
            }
            taken.push(binding.key);
            if binding.hint.is_some() {
                result.push(binding);
            }
        }
        if context.exclusive() {
            break;
        }
    }
    result
}
```

In `src/lib.rs` add `pub mod input;`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test keymap_tests`
Expected: PASS (11 tests). `BINDINGS` order is the footer order: `Q QUIT` is the first `Global` entry so it survives footer truncation. Do not sort the table.

- [ ] **Step 7: Run all checks and commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
git add src tests
git commit -m "feat: add context-aware keymap, actions and list state"
```

---

### Task 6: App'i Action/Effect modeline bağlama

Hata 1–4, `Tab` kısmı (hata 8) ve bağlama duyarlı footer (hata 9) burada kapanır.

**Files:**
- Modify: `src/app.rs` (tamamen yeniden yazılır; içindeki `mod tests` silinir), `src/ui/state.rs` (`UiState` yeniden yazılır; `UiCommand` silinir), `src/ui/mod.rs` (`RenderOutput`), `src/ui/widgets.rs`
- Test: `tests/app_tests.rs` (new), `tests/ui_tests.rs`

**Interfaces:**
- Consumes: Task 5 `Action`, `Effect`, `Context`, `resolve`, `hints`, `ListState`, `PanelId`, `Workspace::panels`.
- Produces (`filiz::ui`): `RenderOutput { viewports: HashMap<PanelId, usize> }` (`Clone, Debug, Default`); `render(frame, &App) -> RenderOutput`.
- Produces (`filiz::ui::state`): `UiState { workspace, focus: PanelId, density, hidden_panels: HashSet<PanelId>, lists: HashMap<PanelId, ListState>, menu_open, theme }`; `set_workspace`, `visible_panels() -> Vec<PanelId>`, `focus_step(isize)`, `toggle_panel() -> PanelToggle`, `list(PanelId) -> ListState`, `list_mut(PanelId) -> &mut ListState`; `PanelToggle::{Hidden(PanelId), Restored, LastPanel}`.
- Produces (`filiz::app`): `App::{new, apply_update, pump, contexts, handle_key(KeyEvent) -> Vec<Effect>, handle_mouse(MouseEvent) -> Vec<Effect>, update(Action) -> Vec<Effect>, visible_processes, selected_process, selected_interface, traffic_rows, list_len, viewport}`; `App.last_render: RenderOutput`.
- Removed: `app::Panel`, `AppCommand`, `App.focus`, `App.selected_index`, `UiCommand`, `UiState::{handle_key, handle_mouse, scroll_by, scroll_offsets, network_interface}`.

- [ ] **Step 1: Write the failing tests**

`tests/app_tests.rs`:

```rust
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use filiz::app::App;
use filiz::input::action::Effect;
use filiz::model::{ActionKind, AppMode, PendingAction, ProcessIdentity, ProcessInfo, SortMode};
use filiz::state::{CollectorUpdate, InterfaceStats, SystemSample};
use filiz::ui::state::{PanelId, Workspace};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn process(pid: u32, cpu: f32) -> ProcessInfo {
    ProcessInfo {
        identity: ProcessIdentity {
            pid,
            start_time: u64::from(pid) * 100,
        },
        name: format!("process-{pid}"),
        command: format!("/bin/process-{pid}"),
        cpu_percent: Some(cpu),
        memory_bytes: Some(u64::from(pid) * 1024),
        user: None,
        status: None,
        traffic: None,
    }
}

fn iface(name: &str) -> InterfaceStats {
    InterfaceStats {
        name: name.into(),
        rx_rate: Some(1.0),
        tx_rate: Some(1.0),
        rx_total: 0,
        tx_total: 0,
        peak_rx: 0.0,
        peak_tx: 0.0,
    }
}

fn app_with(processes: Vec<ProcessInfo>, interfaces: Vec<InterfaceStats>) -> App {
    let mut app = App::new(Duration::from_secs(2));
    app.apply_update(CollectorUpdate::System(SystemSample {
        processes,
        interfaces,
        ..Default::default()
    }));
    app
}

fn app_with_processes() -> App {
    app_with(vec![process(20, 10.0), process(10, 20.0)], Vec::new())
}

#[test]
fn q_and_ctrl_c_quit() {
    let mut app = app_with_processes();
    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), vec![Effect::Quit]);
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        vec![Effect::Quit]
    );
}

#[test]
fn tab_cycles_only_visible_panels_of_current_workspace() {
    let mut app = app_with_processes();
    assert_eq!(app.ui.focus, PanelId::Processes);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.ui.focus, PanelId::Details);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.ui.focus, PanelId::Resources);
    app.handle_key(key(KeyCode::Char('3')));
    assert_eq!(app.ui.focus, PanelId::Interfaces);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.ui.focus, PanelId::Traffic);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.ui.focus, PanelId::Interfaces);
}

#[test]
fn hiding_a_panel_moves_focus_and_h_restores() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Char('h')));
    assert!(app.ui.hidden_panels.contains(&PanelId::Details));
    assert_ne!(app.ui.focus, PanelId::Details);
    for _ in 0..3 {
        app.handle_key(key(KeyCode::Tab));
        assert_ne!(app.ui.focus, PanelId::Details);
    }
    app.handle_key(key(KeyCode::Char('h')));
    assert!(app.ui.hidden_panels.is_empty());
}

#[test]
fn last_visible_panel_cannot_be_hidden() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Char('4')));
    app.handle_key(key(KeyCode::Char('h')));
    assert!(app.ui.hidden_panels.is_empty());
    assert!(app.notice.as_deref().unwrap().contains("cannot be hidden"));
}

#[test]
fn arrows_select_sorted_processes_without_leaving_bounds() {
    let mut app = app_with_processes();
    assert_eq!(app.selected_process().unwrap().identity.pid, 10);
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.selected_process().unwrap().identity.pid, 20);
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.selected_process().unwrap().identity.pid, 20);
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.selected_process().unwrap().identity.pid, 10);
}

#[test]
fn page_down_moves_by_rendered_viewport() {
    let processes = (1..=20).map(|pid| process(pid, 1.0)).collect();
    let mut app = app_with(processes, Vec::new());
    app.last_render.viewports.insert(PanelId::Processes, 5);
    app.handle_key(key(KeyCode::PageDown));
    assert_eq!(app.ui.list(PanelId::Processes).selected, 5);
    app.handle_key(key(KeyCode::End));
    assert_eq!(app.ui.list(PanelId::Processes).selected, 19);
    assert_eq!(app.ui.list(PanelId::Processes).offset, 15);
    app.handle_key(key(KeyCode::Home));
    assert_eq!(app.ui.list(PanelId::Processes), Default::default());
}

#[test]
fn m_toggles_menu_without_changing_sort_and_s_cycles_sort() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Char('m')));
    assert!(app.ui.menu_open);
    assert_eq!(app.sort, SortMode::Cpu);
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.ui.menu_open);
    app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(app.sort, SortMode::Memory);
    assert_eq!(app.visible_processes()[0].identity.pid, 20);
    assert_eq!(app.selected_process().unwrap().identity.pid, 10, "selection follows identity");
}

#[test]
fn enter_opens_detail_and_escape_closes_it() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.mode, AppMode::ProcessDetail);
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.mode, AppMode::Dashboard);
}

#[test]
fn confirmation_requires_explicit_yes() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.mode, AppMode::ConfirmingAction);
    assert!(app.handle_key(key(KeyCode::Enter)).is_empty());
    assert!(app.handle_key(key(KeyCode::Char('q'))).is_empty());
    assert_eq!(
        app.handle_key(key(KeyCode::Char('y'))),
        vec![Effect::SendSignal(
            PendingAction::new(process(10, 20.0).identity, ActionKind::Terminate).confirm()
        )]
    );
    assert_eq!(app.mode, AppMode::Dashboard);
}

#[test]
fn shift_k_requests_kill_and_n_cancels() {
    let mut app = app_with_processes();
    app.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT));
    assert_eq!(app.pending_action.unwrap().kind(), ActionKind::Kill);
    assert!(app.handle_key(key(KeyCode::Char('n'))).is_empty());
    assert!(app.pending_action.is_none());
    assert_eq!(app.mode, AppMode::Dashboard);
}

#[test]
fn filter_mode_keeps_typed_text_including_q() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Char('f')));
    for c in ['q', '2', 'İ'] {
        assert!(app.handle_key(key(KeyCode::Char(c))).is_empty());
    }
    assert_eq!(app.filter, "q2İ");
    app.handle_key(key(KeyCode::Backspace));
    app.handle_key(key(KeyCode::Backspace));
    app.handle_key(key(KeyCode::Backspace));
    app.handle_key(key(KeyCode::Char('2')));
    assert_eq!(app.visible_processes().len(), 1);
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.mode, AppMode::Dashboard);
    app.handle_key(key(KeyCode::Esc));
    assert!(app.filter.is_empty());
}

#[test]
fn network_arrows_move_interfaces_not_processes() {
    let mut app = app_with(
        vec![process(20, 10.0), process(10, 20.0)],
        vec![iface("en0"), iface("en1"), iface("utun3")],
    );
    app.handle_key(key(KeyCode::Char('3')));
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.selected_interface().unwrap().name, "utun3");
    assert_eq!(app.selected_process().unwrap().identity.pid, 10);
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.selected_interface().unwrap().name, "en1");
}

#[test]
fn interface_selection_clamps_when_interface_disappears() {
    let mut app = app_with(Vec::new(), vec![iface("en0"), iface("en1"), iface("utun3")]);
    app.update(filiz::input::action::Action::GoWorkspace(Workspace::Network));
    app.handle_key(key(KeyCode::End));
    app.apply_update(CollectorUpdate::System(SystemSample {
        interfaces: vec![iface("en0")],
        ..Default::default()
    }));
    assert_eq!(app.selected_interface().unwrap().name, "en0");
}

#[test]
fn selection_is_kept_by_full_identity() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Down));
    app.apply_update(CollectorUpdate::System(SystemSample {
        processes: vec![process(20, 80.0), process(10, 1.0)],
        ..Default::default()
    }));
    assert_eq!(app.selected_process().unwrap().identity.pid, 20);
    let mut reused = process(20, 80.0);
    reused.identity.start_time += 1;
    app.apply_update(CollectorUpdate::System(SystemSample {
        processes: vec![reused, process(10, 1.0)],
        ..Default::default()
    }));
    assert_eq!(app.selected_process().unwrap().identity.pid, 10);
}

#[test]
fn process_exit_during_confirmation_cancels_it() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Char('k')));
    app.apply_update(CollectorUpdate::System(SystemSample {
        processes: vec![process(20, 10.0)],
        ..Default::default()
    }));
    assert_eq!(app.mode, AppMode::Dashboard);
    assert!(app.pending_action.is_none());
    assert!(app.notice.as_deref().unwrap().contains("no longer available"));
}

#[test]
fn self_process_does_not_enter_action_confirmation() {
    let mut app = app_with(vec![process(std::process::id(), 1.0)], Vec::new());
    assert!(app.handle_key(key(KeyCode::Char('k'))).is_empty());
    assert_eq!(app.mode, AppMode::Dashboard);
    assert!(app.notice.as_deref().unwrap().contains("itself"));
}

#[test]
fn r_requests_refresh_effect() {
    let mut app = app_with_processes();
    assert_eq!(app.handle_key(key(KeyCode::Char('r'))), vec![Effect::Refresh]);
}
```

Run: `cargo test --test app_tests`
Expected: FAIL — `no field ui.focus`, `handle_key` returns `AppCommand`.

- [ ] **Step 2: Rewrite `UiState` in `src/ui/state.rs`**

Keep `Workspace` (with `ALL`, `label`, `step`, `panels`; delete `from_number`), `LayoutDensity`, `PanelId`. Delete `UiCommand` and the old `UiState`. Imports:

```rust
use std::collections::{HashMap, HashSet};

use super::theme::Theme;
use crate::input::list::ListState;
```

Add:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelToggle {
    Hidden(PanelId),
    Restored,
    LastPanel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiState {
    pub workspace: Workspace,
    pub focus: PanelId,
    pub density: LayoutDensity,
    pub hidden_panels: HashSet<PanelId>,
    pub lists: HashMap<PanelId, ListState>,
    pub menu_open: bool,
    pub theme: Theme,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            workspace: Workspace::Overview,
            focus: PanelId::Processes,
            density: LayoutDensity::Balanced,
            hidden_panels: HashSet::new(),
            lists: HashMap::new(),
            menu_open: false,
            theme: Theme::Forest,
        }
    }
}

impl UiState {
    pub fn visible_panels(&self) -> Vec<PanelId> {
        self.workspace
            .panels()
            .iter()
            .copied()
            .filter(|panel| !self.hidden_panels.contains(panel))
            .collect()
    }

    pub fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
        self.menu_open = false;
        let visible = self.visible_panels();
        if !visible.contains(&self.focus) {
            self.focus = visible.first().copied().unwrap_or(workspace.panels()[0]);
        }
    }

    pub fn focus_step(&mut self, direction: isize) {
        let visible = self.visible_panels();
        if visible.is_empty() {
            return;
        }
        let current = visible.iter().position(|panel| *panel == self.focus).unwrap_or(0) as isize;
        let next = (current + direction).rem_euclid(visible.len() as isize) as usize;
        self.focus = visible[next];
    }

    /// `H`: restore this workspace's hidden panels, or hide the focused one.
    pub fn toggle_panel(&mut self) -> PanelToggle {
        let panels = self.workspace.panels();
        if panels.iter().any(|panel| self.hidden_panels.contains(panel)) {
            for panel in panels {
                self.hidden_panels.remove(panel);
            }
            return PanelToggle::Restored;
        }
        if self.visible_panels().len() <= 1 {
            return PanelToggle::LastPanel;
        }
        let hidden = self.focus;
        self.hidden_panels.insert(hidden);
        self.focus = self.visible_panels()[0];
        PanelToggle::Hidden(hidden)
    }

    pub fn list(&self, panel: PanelId) -> ListState {
        self.lists.get(&panel).copied().unwrap_or_default()
    }

    pub fn list_mut(&mut self, panel: PanelId) -> &mut ListState {
        self.lists.entry(panel).or_default()
    }
}
```

- [ ] **Step 3: Add `RenderOutput` to `src/ui/mod.rs`**

```rust
use std::collections::HashMap;

use state::PanelId;

/// Layout facts from the last frame that input handling needs.
#[derive(Clone, Debug, Default)]
pub struct RenderOutput {
    pub viewports: HashMap<PanelId, usize>,
}
```

Change `pub fn render(frame: &mut Frame, app: &App)` to return `RenderOutput`: create `let mut output = RenderOutput::default();` at the top, pass `&mut output` to `widgets::processes`, `widgets::network`, `widgets::disks`, and `return output;` at both exits (the early `return;` in the Network/Disks/More branch becomes `return output;`). Replace `crate::app::Panel::Resources/Details/Processes` with `PanelId::Resources/Details/Processes`. Delete the `mod tests` block in `src/ui/mod.rs` (its cases are covered by `tests/ui_tests.rs`).

- [ ] **Step 4: Replace `src/app.rs` entirely**

```rust
use std::io::Stdout;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::actions::ProcessAction;
use crate::collectors::runtime::CollectorRuntime;
use crate::history::History;
use crate::input::action::{Action, Effect};
use crate::input::keymap::{self, Context};
use crate::input::list::ListState;
use crate::model::{
    filter_processes, sort_processes, ActionKind, AppMode, PendingAction, ProcessIdentity,
    ProcessInfo, SortMode,
};
use crate::state::{CollectorUpdate, InterfaceStats, SystemState};
use crate::ui::state::{PanelId, PanelToggle, UiState, Workspace};
use crate::ui::{self, RenderOutput};

pub struct App {
    pub refresh: Duration,
    pub state: SystemState,
    pub history: History,
    pub mode: AppMode,
    pub sort: SortMode,
    pub filter: String,
    pub pending_action: Option<PendingAction>,
    pub notice: Option<String>,
    pub ui: UiState,
    pub last_render: RenderOutput,
    selected_identity: Option<ProcessIdentity>,
    notice_until: Option<Instant>,
}

impl App {
    pub fn new(refresh: Duration) -> Self {
        Self {
            refresh,
            state: SystemState::default(),
            history: History::new(60),
            mode: AppMode::Dashboard,
            sort: SortMode::Cpu,
            filter: String::new(),
            pending_action: None,
            notice: None,
            ui: UiState::default(),
            last_render: RenderOutput::default(),
            selected_identity: None,
            notice_until: None,
        }
    }

    pub fn apply_update(&mut self, update: CollectorUpdate) {
        let is_system = matches!(update, CollectorUpdate::System(_));
        self.state.apply(update);
        if is_system {
            self.history.record(&self.state);
        }
        self.reconcile_selection();
        for panel in [PanelId::Interfaces, PanelId::Traffic, PanelId::Disks] {
            let (len, viewport) = (self.list_len(panel), self.viewport(panel));
            self.ui.list_mut(panel).clamp(len, viewport);
        }
    }

    /// Apply every queued collector update without blocking. Returns true if anything changed.
    pub fn pump(&mut self, updates: &Receiver<CollectorUpdate>) -> bool {
        let mut changed = false;
        while let Ok(update) = updates.try_recv() {
            self.apply_update(update);
            changed = true;
        }
        changed
    }

    pub fn visible_processes(&self) -> Vec<ProcessInfo> {
        let mut processes = filter_processes(&self.state.processes, &self.filter);
        sort_processes(&mut processes, self.sort);
        processes
    }

    pub fn selected_process(&self) -> Option<ProcessInfo> {
        self.visible_processes()
            .get(self.ui.list(PanelId::Processes).selected)
            .cloned()
    }

    pub fn selected_interface(&self) -> Option<&InterfaceStats> {
        self.state
            .interfaces
            .get(self.ui.list(PanelId::Interfaces).selected)
    }

    /// Processes with measured traffic, busiest first.
    pub fn traffic_rows(&self) -> Vec<&ProcessInfo> {
        let mut rows: Vec<&ProcessInfo> = self
            .state
            .processes
            .iter()
            .filter(|process| process.traffic.is_some())
            .collect();
        let total = |process: &ProcessInfo| process.traffic.map_or(0.0, |t| t.rx + t.tx);
        rows.sort_by(|a, b| total(b).total_cmp(&total(a)));
        rows
    }

    pub fn list_len(&self, panel: PanelId) -> usize {
        match panel {
            PanelId::Processes => self.visible_processes().len(),
            PanelId::Interfaces => self.state.interfaces.len(),
            PanelId::Traffic => self.traffic_rows().len(),
            PanelId::Disks => self.state.visible_disks().len(),
            PanelId::Resources | PanelId::Details | PanelId::Settings => 0,
        }
    }

    pub fn viewport(&self, panel: PanelId) -> usize {
        self.last_render
            .viewports
            .get(&panel)
            .copied()
            .unwrap_or(1)
            .max(1)
    }

    pub fn contexts(&self) -> Vec<Context> {
        match self.mode {
            AppMode::ConfirmingAction => vec![Context::Confirm],
            AppMode::Filtering => vec![Context::Filter],
            AppMode::ProcessDetail => vec![Context::Detail, Context::Global],
            AppMode::Dashboard => {
                let mut stack = Vec::new();
                if self.ui.menu_open {
                    stack.push(Context::Menu);
                }
                stack.push(Context::Panel(self.ui.focus));
                stack.push(Context::Workspace(self.ui.workspace));
                stack.push(Context::Global);
                stack
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match keymap::resolve(&self.contexts(), &key) {
            Some(action) => self.update(action),
            None => Vec::new(),
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Vec<Effect> {
        if self.mode != AppMode::Dashboard {
            return Vec::new();
        }
        let delta = match mouse.kind {
            MouseEventKind::ScrollUp => -3,
            MouseEventKind::ScrollDown => 3,
            _ => return Vec::new(),
        };
        self.update(Action::Scroll(self.ui.focus, delta))
    }

    pub fn update(&mut self, action: Action) -> Vec<Effect> {
        let focus = self.ui.focus;
        match action {
            Action::Quit => return vec![Effect::Quit],
            Action::Refresh => return vec![Effect::Refresh],
            Action::GoWorkspace(workspace) => self.go_workspace(workspace),
            Action::NextWorkspace => self.go_workspace(self.ui.workspace.step(1)),
            Action::PrevWorkspace => self.go_workspace(self.ui.workspace.step(-1)),
            Action::FocusNext => self.ui.focus_step(1),
            Action::FocusPrev => self.ui.focus_step(-1),
            Action::Focus(panel) => {
                if self.ui.visible_panels().contains(&panel) {
                    self.ui.focus = panel;
                }
            }
            Action::MoveUp => self.move_list(focus, -1),
            Action::MoveDown => self.move_list(focus, 1),
            Action::PageUp => self.move_list(focus, -(self.viewport(focus) as isize)),
            Action::PageDown => self.move_list(focus, self.viewport(focus) as isize),
            Action::Home => self.move_list(focus, isize::MIN / 2),
            Action::End => self.move_list(focus, isize::MAX / 2),
            Action::Scroll(panel, delta) => self.move_list(panel, isize::from(delta)),
            Action::Select(panel, index) => {
                self.ui.focus = panel;
                let current = self.ui.list(panel).selected as isize;
                self.move_list(panel, index as isize - current);
            }
            Action::Open => {
                if focus == PanelId::Processes && self.selected_process().is_some() {
                    self.mode = AppMode::ProcessDetail;
                }
            }
            Action::Back => self.back(),
            Action::StartFilter => self.mode = AppMode::Filtering,
            Action::FilterInput(character) => {
                self.filter.push(character);
                self.select_first();
            }
            Action::FilterBackspace => {
                self.filter.pop();
                self.select_first();
            }
            Action::FilterSubmit => self.mode = AppMode::Dashboard,
            Action::CycleSort => {
                self.sort = match self.sort {
                    SortMode::Cpu => SortMode::Memory,
                    SortMode::Memory => SortMode::Cpu,
                };
                self.reconcile_selection();
            }
            Action::Terminate => self.begin_action(ActionKind::Terminate),
            Action::Kill => self.begin_action(ActionKind::Kill),
            Action::Confirm => {
                self.mode = AppMode::Dashboard;
                if let Some(pending) = self.pending_action.take() {
                    return vec![Effect::SendSignal(pending.confirm())];
                }
            }
            Action::Cancel => {
                self.mode = AppMode::Dashboard;
                if let Some(pending) = self.pending_action.take() {
                    let _ = pending.cancel();
                }
            }
            Action::ToggleMenu => self.ui.menu_open = !self.ui.menu_open,
            Action::CycleTheme => {
                self.ui.theme = self.ui.theme.next();
                self.show_notice(format!("Theme: {}", self.ui.theme.label()));
            }
            Action::CycleDensity => {
                self.ui.density = self.ui.density.cycle();
                self.show_notice(format!("Layout: {:?}", self.ui.density));
            }
            Action::TogglePanel => {
                let message = match self.ui.toggle_panel() {
                    PanelToggle::Hidden(panel) => {
                        format!("{} panel hidden. Press H to restore.", panel.label())
                    }
                    PanelToggle::Restored => "All panels visible.".to_owned(),
                    PanelToggle::LastPanel => "The last visible panel cannot be hidden.".to_owned(),
                };
                self.show_notice(message);
            }
        }
        Vec::new()
    }

    fn go_workspace(&mut self, workspace: Workspace) {
        self.ui.set_workspace(workspace);
        self.show_notice(format!("Workspace: {}", workspace.label()));
    }

    fn back(&mut self) {
        if self.mode == AppMode::ProcessDetail {
            self.mode = AppMode::Dashboard;
        } else if self.ui.menu_open {
            self.ui.menu_open = false;
        } else if !self.filter.is_empty() {
            self.filter.clear();
            self.select_first();
        }
    }

    fn move_list(&mut self, panel: PanelId, delta: isize) {
        let (len, viewport) = (self.list_len(panel), self.viewport(panel));
        self.ui.list_mut(panel).move_by(delta, len, viewport);
        if panel == PanelId::Processes {
            self.selected_identity = self.selected_process().map(|process| process.identity);
        }
    }

    fn begin_action(&mut self, kind: ActionKind) {
        let Some(process) = self.selected_process() else {
            return;
        };
        if process.identity.pid == std::process::id() {
            self.show_notice("Filiz cannot send a signal to itself.");
            return;
        }
        self.pending_action = Some(PendingAction::new(process.identity, kind));
        self.mode = AppMode::ConfirmingAction;
    }

    fn select_first(&mut self) {
        *self.ui.list_mut(PanelId::Processes) = ListState::default();
        self.selected_identity = self.visible_processes().first().map(|p| p.identity);
    }

    fn reconcile_selection(&mut self) {
        let processes = self.visible_processes();
        let previous = self.selected_identity;
        let index = previous
            .and_then(|identity| processes.iter().position(|p| p.identity == identity))
            .or_else(|| {
                processes
                    .iter()
                    .position(|p| previous.is_none_or(|identity| p.identity.pid != identity.pid))
            })
            .unwrap_or(0);
        let viewport = self.viewport(PanelId::Processes);
        let list = self.ui.list_mut(PanelId::Processes);
        list.selected = index;
        list.clamp(processes.len(), viewport);
        self.selected_identity = processes.get(list.selected).map(|p| p.identity);
        if self.mode == AppMode::ConfirmingAction
            && self
                .pending_action
                .is_some_and(|pending| !processes.iter().any(|p| p.identity == pending.identity()))
        {
            self.pending_action = None;
            self.mode = AppMode::Dashboard;
            self.show_notice("Selected process is no longer available.");
        }
        if self.mode == AppMode::ProcessDetail && self.selected_identity != previous {
            self.mode = AppMode::Dashboard;
        }
    }

    pub fn show_notice(&mut self, message: impl Into<String>) {
        self.notice = Some(message.into());
        self.notice_until = Some(Instant::now() + Duration::from_secs(5));
    }

    fn clear_expired_notice(&mut self) -> bool {
        if self.notice_until.is_some_and(|until| Instant::now() >= until) {
            self.notice = None;
            self.notice_until = None;
            return true;
        }
        false
    }
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    runtime: CollectorRuntime,
    updates: Receiver<CollectorUpdate>,
) -> Result<()> {
    const INPUT_WAIT: Duration = Duration::from_millis(50);
    let mut dirty = true;
    loop {
        if app.pump(&updates) {
            dirty = true;
        }
        if app.clear_expired_notice() {
            dirty = true;
        }
        if dirty {
            let mut output = RenderOutput::default();
            terminal.draw(|frame| output = ui::render(frame, app))?;
            app.last_render = output;
            dirty = false;
        }
        if !event::poll(INPUT_WAIT)? {
            continue;
        }
        let effects = match event::read()? {
            Event::Key(key) => app.handle_key(key),
            Event::Mouse(mouse) => app.handle_mouse(mouse),
            Event::Resize(_, _) => Vec::new(),
            _ => continue,
        };
        dirty = true;
        for effect in effects {
            match effect {
                Effect::Quit => return Ok(()),
                Effect::Refresh => runtime.refresh(),
                Effect::SendSignal(action) => {
                    let pid = action.identity().pid;
                    match ProcessAction::execute(action) {
                        Ok(()) => app.show_notice(format!("Signal sent to PID {pid}.")),
                        Err(error) => app.show_notice(error.to_user_message()),
                    }
                    runtime.refresh();
                }
            }
        }
    }
}
```

- [ ] **Step 5: Update `src/ui/widgets.rs`**

- Imports: replace `use crate::app::{App, Panel};` with `use crate::app::App;`, add `use super::state::PanelId;`, `use super::RenderOutput;`, `use crate::input::action::Action;`, `use crate::input::keymap;`. Delete `fn selected_interface` (use `app.selected_interface()`).
- Replace every `app.focus == Panel::X` with `app.ui.focus == PanelId::X`.
- Add a helper used by every list table:

```rust
fn table_state(app: &App, panel: PanelId, len: usize) -> TableState {
    let list = app.ui.list(panel);
    let state = TableState::default().with_offset(list.offset);
    if len == 0 {
        state
    } else {
        state.with_selected(Some(list.selected))
    }
}

fn border_for(app: &App, panel: PanelId, palette: &theme::Palette) -> Style {
    Style::default().fg(if app.ui.focus == panel {
        palette.border_focus
    } else {
        palette.border
    })
}
```

- `processes(frame, area, app, out: &mut RenderOutput)`: at the start insert `out.viewports.insert(PanelId::Processes, area.height.saturating_sub(3) as usize);`. Title becomes `format!(" PROCESSES  {} ", processes.len())`. Header CPU/MEMORY cells become `if app.sort == SortMode::Cpu { "CPU ▼" } else { "CPU" }` and `if app.sort == SortMode::Memory { "MEMORY ▼" } else { "MEMORY" }` (build the header `Row` from a `Vec<&str>`). Replace the `TableState` block with `let mut state = table_state(app, PanelId::Processes, processes.len());`. Row highlight style: `.fg(palette.selection_fg).bg(palette.selection_bg)`.
- `network(frame, area, app, out)`: `let selected = app.selected_interface();`. Interfaces table: `.block(... .border_style(border_for(app, PanelId::Interfaces, &palette)))`, `.row_highlight_style(Style::default().fg(palette.selection_fg).bg(palette.selection_bg))`, `.highlight_symbol("▸ ")`, render with `frame.render_stateful_widget(table, sections[1], &mut table_state(app, PanelId::Interfaces, app.state.interfaces.len()))`, and `out.viewports.insert(PanelId::Interfaces, sections[1].height.saturating_sub(3) as usize);`. Traffic table: `let talkers = app.traffic_rows();` (delete the local sort and the `scroll_offsets` offset/skip), rows are `talkers.iter().map(|p| traffic_row(p))`, border `border_for(app, PanelId::Traffic, &palette)`, same highlight style and symbol, `render_stateful_widget(..., &mut table_state(app, PanelId::Traffic, talkers.len()))`, `out.viewports.insert(PanelId::Traffic, sections[2].height.saturating_sub(3) as usize);`.
- `disks(frame, area, app, out)`: border `border_for(app, PanelId::Disks, &palette)`, highlight style and symbol as above, stateful render with `table_state(app, PanelId::Disks, disks.len())`, `out.viewports.insert(PanelId::Disks, area.height.saturating_sub(3) as usize);`.
- Replace `pub fn footer` entirely:

```rust
pub fn footer(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
    let max_width = area.width as usize;
    let mut lines: Vec<Vec<Span>> = vec![Vec::new()];
    let mut width = 0;
    if app.mode == AppMode::Filtering {
        let prompt = format!(" {}_  ", app.filter);
        width = 8 + prompt.chars().count();
        lines[0].push(Span::styled(
            " FILTER ",
            Style::default().fg(palette.accent_fg).bg(palette.accent),
        ));
        lines[0].push(Span::styled(prompt, Style::default().fg(palette.text)));
    }
    for binding in keymap::hints(&app.contexts()) {
        let hint = binding.hint.unwrap_or_default();
        let needed = binding.label.chars().count() + hint.chars().count() + 4;
        if width + needed > max_width {
            if lines.len() == area.height as usize {
                break;
            }
            lines.push(Vec::new());
            width = 0;
        }
        width += needed;
        let key_color = if matches!(binding.action, Action::Terminate | Action::Kill) {
            palette.warn
        } else {
            palette.accent
        };
        let line = lines.last_mut().expect("footer line");
        line.push(Span::styled(
            format!("  {} ", binding.label),
            Style::default().fg(key_color).add_modifier(Modifier::BOLD),
        ));
        line.push(Span::styled(
            format!("{hint} "),
            Style::default().fg(palette.text_muted),
        ));
    }
    let lines: Vec<Line> = lines.into_iter().map(Line::from).collect();
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(palette.surface)),
        area,
    );
}
```

- `more`: the `MENU OPEN` line text becomes `"  MENU OPEN  [L] density  [T] theme  [Esc] close"`.

- [ ] **Step 6: Update `tests/ui_tests.rs`**

Change the import to `use filiz::ui::{render, state::{PanelId, Workspace}};` and `screen` to ignore the returned `RenderOutput` (`terminal.draw(|frame| { render(frame, app); })`). Delete `navigation_switches_workspaces_and_cycles_density` and `panel_toggle_and_scroll_are_scoped_to_the_focused_panel` (covered by `app_tests`). Change `dashboard_renders_without_panic_when_compact` to assert `"FILIZ"`, `"PROCESSES"` and `"QUIT"` at 48×20. Add:

```rust
#[test]
fn footer_shows_only_keys_of_the_active_context() {
    let mut app = sample_app();
    let processes = screen(&app, 110, 35);
    assert!(processes.contains("FILTER"));
    app.ui.set_workspace(Workspace::Network);
    let network = screen(&app, 110, 35);
    assert!(!network.contains("FILTER"));
    assert!(network.contains("QUIT"));
    app.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
    app.ui.focus = PanelId::Interfaces;
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    assert!(app.pending_action.is_none());
}

#[test]
fn sort_column_is_marked() {
    let mut app = sample_app();
    assert!(screen(&app, 110, 35).contains("CPU ▼"));
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    assert!(screen(&app, 110, 35).contains("MEMORY ▼"));
}
```

- [ ] **Step 7: Run all checks and commit**

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: PASS (17 new app tests, 2 new UI tests).

Manual: `cargo run --release` → `3`, `↓` moves interface highlight; `Tab` → traffic list focused; `M` opens menu, `S` sorts, footer changes per workspace.

```bash
git add src tests
git commit -m "feat: route input through context-aware keymap and actions"
```

---

### Task 7: Bileşen ve workspace ayrımı, Processes düzeni

`widgets.rs` bölünür. Hata 7 (Processes = Overview kopyası) ve hata 8'in kalan kısmı (`H`/`L` yalnızca Overview'da) kapanır. Spec'teki `WorkspaceView::panels` yerine panel listesi Task 5'te `Workspace::panels()` olarak kuruldu; trait yalnızca `render` taşır.

**Files:**
- Create: `src/ui/components/{mod.rs,status.rs,card.rs,table.rs,modal.rs,footer.rs}`, `src/ui/workspaces/{mod.rs,overview.rs,processes.rs,network.rs,disks.rs,more.rs}`
- Modify: `src/ui/mod.rs`
- Delete: `src/ui/widgets.rs`
- Test: `tests/ui_tests.rs`

**Interfaces:**
- Consumes: `App`, `RenderOutput`, `PanelId`, `Workspace`, `LayoutDensity`, `Palette`.
- Produces (`filiz::ui`): `RenderCx<'a> { app: &'a App, palette: Palette, frame_height: u16, out: &'a mut RenderOutput }` with `visible(PanelId) -> bool`, `focused(PanelId) -> bool`.
- Produces (`filiz::ui::workspaces`): `trait WorkspaceView { fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx); }`, `view(Workspace) -> &'static dyn WorkspaceView`.
- Produces (`filiz::ui::components::table`): `table_state(&App, PanelId, usize) -> TableState`, `border_for(&RenderCx, PanelId) -> Style`, `highlight(&Palette) -> Style`.
- Task 8 extends `RenderOutput` and `RenderCx` with a `HitMap`.

- [ ] **Step 1: Write the failing tests**

Add to `tests/ui_tests.rs`:

```rust
#[test]
fn processes_workspace_has_its_own_layout() {
    let mut app = sample_app();
    let overview = screen(&app, 110, 35);
    assert!(overview.contains("CORES"), "overview shows resource cards");
    app.ui.set_workspace(Workspace::Processes);
    let processes = screen(&app, 110, 35);
    assert!(!processes.contains("CORES"));
    assert!(processes.contains("PROCESSES"));
    assert!(processes.contains("DETAILS / EVENTS"));
    assert!(processes.contains("example-worker"));
}

#[test]
fn hide_works_in_every_workspace() {
    let mut app = sample_app();
    app.ui.set_workspace(Workspace::Processes);
    app.ui.focus = PanelId::Details;
    app.ui.toggle_panel();
    assert!(!screen(&app, 110, 35).contains("DETAILS / EVENTS"));

    app.ui.set_workspace(Workspace::Network);
    app.ui.focus = PanelId::Traffic;
    app.ui.toggle_panel();
    let network = screen(&app, 110, 35);
    assert!(!network.contains("PROCESS TRAFFIC"));
    assert!(network.contains("DOWNLOAD"));
}

#[test]
fn every_workspace_density_and_size_renders_with_scrolled_lists() {
    use filiz::input::list::ListState;
    use filiz::ui::state::LayoutDensity;
    for workspace in Workspace::ALL {
        for density in [LayoutDensity::Compact, LayoutDensity::Balanced, LayoutDensity::Spacious] {
            for (width, height) in [(110, 35), (80, 24), (48, 20), (20, 8), (10, 3)] {
                let mut app = sample_app();
                app.ui.set_workspace(workspace);
                app.ui.density = density;
                for panel in workspace.panels() {
                    app.ui.lists.insert(*panel, ListState { selected: 500, offset: 400 });
                }
                let _ = screen(&app, width, height);
            }
        }
    }
}
```

Run: `cargo test --test ui_tests processes_workspace_has_its_own_layout`
Expected: FAIL — Processes workspace still renders the Overview cards (`CORES` present).

- [ ] **Step 2: Create `src/ui/components/table.rs`**

```rust
use ratatui::style::{Modifier, Style};
use ratatui::widgets::TableState;

use crate::app::App;
use crate::ui::state::PanelId;
use crate::ui::theme::Palette;
use crate::ui::RenderCx;

pub fn table_state(app: &App, panel: PanelId, len: usize) -> TableState {
    let list = app.ui.list(panel);
    let state = TableState::default().with_offset(list.offset.min(len.saturating_sub(1)));
    if len == 0 {
        state
    } else {
        state.with_selected(Some(list.selected.min(len - 1)))
    }
}

pub fn border_for(cx: &RenderCx, panel: PanelId) -> Style {
    Style::default().fg(if cx.focused(panel) {
        cx.palette.border_focus
    } else {
        cx.palette.border
    })
}

pub fn highlight(palette: &Palette) -> Style {
    Style::default()
        .fg(palette.selection_fg)
        .bg(palette.selection_bg)
        .add_modifier(Modifier::BOLD)
}
```

Delete `table_state` and `border_for` from `widgets.rs` as their callers move; use `highlight(&cx.palette)` wherever a table set `row_highlight_style` inline.

- [ ] **Step 3: Move the remaining functions**

Every moved function changes its signature from `(frame: &mut Frame, area: Rect, app: &App[, out: &mut RenderOutput])` to `(frame: &mut Frame, area: Rect, cx: &mut RenderCx)`; inside, replace `app` with `cx.app`, `let palette = app.ui.theme.palette();` with `let palette = cx.palette;`, `out.viewports` with `cx.out.viewports`, and `border_for(app, P, &palette)` with `border_for(cx, P)`. Private helpers keep their signatures.

| From `widgets.rs` | To | Visibility |
|---|---|---|
| `status` | `components/status.rs` as `render` | `pub` |
| `resource_card`, `network_card`, `usage_color` | `components/card.rs` | `pub(crate)` |
| `footer` | `components/footer.rs` as `render` | `pub` |
| `detail_modal`, `confirmation_modal`, `centered` | `components/modal.rs` as `detail`, `confirmation`, `centered` | `pub`, `pub`, private |
| `resources`, `details`, `cpu_detail`, `opt_bytes`, `memory_detail`, `disk_detail`, `total_rates` | `workspaces/overview.rs` | `resources`/`details` `pub(crate)` (Processes reuses `details`) |
| `processes`, `process_row` | `workspaces/processes.rs` as `process_table`, `process_row` | `pub(crate)` for `process_table` |
| `network`, `network_row`, `traffic_row`, `rate_or_na` | `workspaces/network.rs` | private helpers |
| `disks`, `disk_row` | `workspaces/disks.rs` | private |
| `more` | `workspaces/more.rs` | private |

`src/ui/components/mod.rs`:

```rust
pub mod card;
pub mod footer;
pub mod modal;
pub mod status;
pub mod table;
```

- [ ] **Step 4: Create `src/ui/workspaces/mod.rs`**

```rust
mod disks;
mod more;
mod network;
mod overview;
mod processes;

use ratatui::{layout::Rect, Frame};

use super::state::Workspace;
use super::RenderCx;

pub trait WorkspaceView {
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx);
}

pub fn view(workspace: Workspace) -> &'static dyn WorkspaceView {
    match workspace {
        Workspace::Overview => &overview::Overview,
        Workspace::Processes => &processes::Processes,
        Workspace::Network => &network::Network,
        Workspace::Disks => &disks::Disks,
        Workspace::More => &more::More,
    }
}
```

- [ ] **Step 5: Implement the workspace layouts**

`workspaces/overview.rs` (plus the moved functions):

```rust
pub struct Overview;

impl WorkspaceView for Overview {
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
        let (resource, detail) = match cx.app.ui.density {
            LayoutDensity::Compact => (7, 3),
            LayoutDensity::Spacious => (15, 8),
            LayoutDensity::Balanced => match cx.frame_height {
                28.. => (12, 6),
                20..=27 => (8, 3),
                17..=19 => (6, 3),
                _ => (2, 0),
            },
        };
        let resource = if cx.visible(PanelId::Resources) { resource } else { 0 };
        let detail = if cx.visible(PanelId::Details) { detail } else { 0 };
        let middle = if cx.visible(PanelId::Processes) {
            Constraint::Min(0)
        } else {
            Constraint::Length(0)
        };
        let [top, center, bottom] = Layout::vertical([
            Constraint::Length(resource),
            middle,
            Constraint::Length(detail),
        ])
        .areas(area);
        resources(frame, top, cx);
        if cx.visible(PanelId::Processes) {
            process_table(frame, center, cx);
        }
        details(frame, bottom, cx);
    }
}
```

(`use super::processes::process_table;`)

`workspaces/processes.rs`:

```rust
pub struct Processes;

impl WorkspaceView for Processes {
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
        let detail = match cx.app.ui.density {
            LayoutDensity::Compact => 3,
            LayoutDensity::Balanced => 6,
            LayoutDensity::Spacious => 8,
        };
        let constraints = match (cx.visible(PanelId::Processes), cx.visible(PanelId::Details)) {
            (true, true) => [Constraint::Min(0), Constraint::Length(detail)],
            (true, false) => [Constraint::Min(0), Constraint::Length(0)],
            _ => [Constraint::Length(0), Constraint::Min(0)],
        };
        let [table, detail_area] = Layout::vertical(constraints).areas(area);
        if cx.visible(PanelId::Processes) {
            process_table(frame, table, cx);
        }
        if cx.visible(PanelId::Details) {
            super::overview::details(frame, detail_area, cx);
        }
    }
}
```

`workspaces/network.rs` — replace the start of the old `network` body (the `sections` split) with:

```rust
pub struct Network;

impl WorkspaceView for Network {
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let cards = match cx.app.ui.density {
            LayoutDensity::Compact => 4,
            LayoutDensity::Balanced => 6,
            LayoutDensity::Spacious => 8,
        };
        let interfaces = (cx.app.state.interfaces.len() as u16 + 3).min(8);
        let (interface_constraint, traffic_constraint) =
            match (cx.visible(PanelId::Interfaces), cx.visible(PanelId::Traffic)) {
                (true, true) => (Constraint::Length(interfaces), Constraint::Min(0)),
                (true, false) => (Constraint::Min(0), Constraint::Length(0)),
                _ => (Constraint::Length(0), Constraint::Min(0)),
            };
        let [card_area, interface_area, traffic_area] = Layout::vertical([
            Constraint::Length(cards),
            interface_constraint,
            traffic_constraint,
        ])
        .areas(area);
        rate_cards(frame, card_area, cx);
        if cx.visible(PanelId::Interfaces) {
            interface_table(frame, interface_area, cx);
        }
        if cx.visible(PanelId::Traffic) {
            traffic_table(frame, traffic_area, cx);
        }
    }
}
```

Split the old `network` body into `rate_cards` (the two `network_card` calls), `interface_table` (was `sections[1]`, now `area`) and `traffic_table` (was `sections[2]`, now `area`), each `fn (frame: &mut Frame, area: Rect, cx: &mut RenderCx)`. Viewport inserts use `area.height.saturating_sub(3)`.

`workspaces/disks.rs` and `workspaces/more.rs`: `pub struct Disks;` / `pub struct More;` whose `WorkspaceView::render` calls the moved `disks` / `more` function with the full `area`.

- [ ] **Step 6: Replace `src/ui/mod.rs`**

```rust
pub mod components;
pub mod format;
pub mod state;
pub mod theme;
pub mod workspaces;

use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::Block,
    Frame,
};

use crate::app::App;
use crate::model::AppMode;
use state::{LayoutDensity, PanelId};
use theme::Palette;

/// Layout facts from the last frame that input handling needs.
#[derive(Clone, Debug, Default)]
pub struct RenderOutput {
    pub viewports: HashMap<PanelId, usize>,
}

pub struct RenderCx<'a> {
    pub app: &'a App,
    pub palette: Palette,
    pub frame_height: u16,
    pub out: &'a mut RenderOutput,
}

impl RenderCx<'_> {
    pub fn visible(&self, panel: PanelId) -> bool {
        !self.app.ui.hidden_panels.contains(&panel)
    }

    pub fn focused(&self, panel: PanelId) -> bool {
        self.app.ui.focus == panel
    }
}

pub fn render(frame: &mut Frame, app: &App) -> RenderOutput {
    let mut output = RenderOutput::default();
    let area = frame.area();
    let palette = app.ui.theme.palette();
    frame.render_widget(Block::default().style(Style::default().bg(palette.bg)), area);
    let header = match app.ui.density {
        LayoutDensity::Compact => 2,
        LayoutDensity::Spacious => 3,
        LayoutDensity::Balanced => match area.height {
            28.. => 3,
            17..=27 => 2,
            _ => 1,
        },
    };
    let footer = if area.height >= 17 { 2 } else { 1 };
    let [header_area, body, footer_area] = Layout::vertical([
        Constraint::Length(header),
        Constraint::Min(0),
        Constraint::Length(footer),
    ])
    .areas(area);
    let mut cx = RenderCx {
        app,
        palette,
        frame_height: area.height,
        out: &mut output,
    };
    components::status::render(frame, header_area, &mut cx);
    workspaces::view(app.ui.workspace).render(frame, body, &mut cx);
    components::footer::render(frame, footer_area, &mut cx);
    match app.mode {
        AppMode::ProcessDetail => components::modal::detail(frame, area, &mut cx),
        AppMode::ConfirmingAction => components::modal::confirmation(frame, area, &mut cx),
        AppMode::Dashboard | AppMode::Filtering => {}
    }
    output
}
```

Delete `src/ui/widgets.rs`.

- [ ] **Step 7: Run all checks and commit**

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: PASS. Every earlier UI assertion still holds; `wc -l src/ui/**/*.rs` shows no file above ~350 lines.

Manual: `cargo run --release` → `2` shows the full-height process table with details below; `H` on details hides it; `L` changes heights in Processes and Network.

```bash
git add -A src tests
git commit -m "refactor: split UI into components and workspaces"
```

---

### Task 8: HitMap ve mouse etkileşimi

Sekmeler, satırlar, paneller ve footer ipuçları tıklanabilir olur; wheel imlecin altındaki paneli kaydırır; modal arka planı kilitler.

**Files:**
- Create: `src/ui/hit.rs`
- Modify: `src/ui/mod.rs` (`pub mod hit;`, `RenderOutput.hits`), `src/ui/components/{status.rs,table.rs,footer.rs,modal.rs}`, `src/ui/workspaces/{overview.rs,processes.rs,network.rs,disks.rs,more.rs}`, `src/app.rs` (`handle_mouse`)
- Test: `tests/hit_tests.rs`

**Interfaces:**
- Consumes: `Action`, `Effect`, `PanelId`, `Workspace`, `RenderCx`, `RenderOutput`.
- Produces (`filiz::ui::hit`): `HitTarget::{Tab(Workspace), Panel(PanelId), Row(PanelId, usize), Button(Action), Blocker}`; `HitMap::{push(Rect, HitTarget), at(u16, u16) -> Option<HitTarget>, panel_at(u16, u16) -> Option<PanelId>, has_blocker() -> bool, entries() -> &[(Rect, HitTarget)]}`; `ClickMemory` (`Default`); `mouse_action(&HitMap, &MouseEvent, focus: PanelId, &mut ClickMemory, Instant) -> Option<Action>`.
- Produces: `RenderOutput.hits: HitMap`; `components::table::register_rows(cx, panel, area, len)`.

- [ ] **Step 1: Write the failing tests**

`tests/hit_tests.rs`:

```rust
use std::time::{Duration, Instant};

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use filiz::app::App;
use filiz::input::action::{Action, Effect};
use filiz::model::{AppMode, ProcessIdentity, ProcessInfo, TrafficRate};
use filiz::state::{CollectorUpdate, SystemSample};
use filiz::ui::hit::{mouse_action, ClickMemory, HitMap, HitTarget};
use filiz::ui::state::{PanelId, Workspace};
use filiz::ui::{render, RenderOutput};
use ratatui::{backend::TestBackend, layout::Rect, Terminal};

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn click(column: u16, row: u16) -> MouseEvent {
    mouse(MouseEventKind::Down(MouseButton::Left), column, row)
}

fn process(pid: u32, cpu: f32, traffic: Option<f64>) -> ProcessInfo {
    ProcessInfo {
        identity: ProcessIdentity { pid, start_time: 1 },
        name: format!("proc-{pid}"),
        command: String::new(),
        cpu_percent: Some(cpu),
        memory_bytes: Some(1),
        user: None,
        status: None,
        traffic: traffic.map(|rx| TrafficRate { rx, tx: 0.0 }),
    }
}

fn app() -> App {
    let mut app = App::new(Duration::from_secs(2));
    app.apply_update(CollectorUpdate::System(SystemSample {
        processes: (1..=30).map(|pid| process(pid, 100.0 - pid as f32, Some(pid as f64))).collect(),
        ..Default::default()
    }));
    app
}

fn draw(app: &mut App) {
    let mut terminal = Terminal::new(TestBackend::new(110, 35)).unwrap();
    let mut output = RenderOutput::default();
    terminal.draw(|frame| output = render(frame, app)).unwrap();
    app.last_render = output;
}

fn find(app: &App, target: HitTarget) -> Rect {
    app.last_render
        .hits
        .entries()
        .iter()
        .find(|(_, candidate)| *candidate == target)
        .map(|(rect, _)| *rect)
        .unwrap_or_else(|| panic!("{target:?} not registered"))
}

#[test]
fn later_entries_win_and_blocker_stops_panel_lookup() {
    let mut hits = HitMap::default();
    hits.push(Rect::new(0, 0, 10, 10), HitTarget::Panel(PanelId::Processes));
    hits.push(Rect::new(0, 2, 10, 1), HitTarget::Row(PanelId::Processes, 0));
    assert_eq!(hits.at(3, 2), Some(HitTarget::Row(PanelId::Processes, 0)));
    assert_eq!(hits.panel_at(3, 5), Some(PanelId::Processes));
    assert_eq!(hits.at(20, 20), None);
    hits.push(Rect::new(0, 0, 10, 10), HitTarget::Blocker);
    assert_eq!(hits.panel_at(3, 5), None);
    assert!(hits.has_blocker());
}

#[test]
fn second_click_on_same_row_within_400ms_opens() {
    let mut hits = HitMap::default();
    hits.push(Rect::new(0, 0, 10, 1), HitTarget::Row(PanelId::Processes, 4));
    let mut memory = ClickMemory::default();
    let now = Instant::now();
    let first = mouse_action(&hits, &click(1, 0), PanelId::Resources, &mut memory, now);
    assert_eq!(first, Some(Action::Select(PanelId::Processes, 4)));
    let second = mouse_action(
        &hits,
        &click(1, 0),
        PanelId::Processes,
        &mut memory,
        now + Duration::from_millis(200),
    );
    assert_eq!(second, Some(Action::Open));
    let late = mouse_action(
        &hits,
        &click(1, 0),
        PanelId::Processes,
        &mut memory,
        now + Duration::from_secs(2),
    );
    assert_eq!(late, Some(Action::Select(PanelId::Processes, 4)));
}

#[test]
fn clicking_a_tab_switches_workspace() {
    let mut app = app();
    draw(&mut app);
    let tab = find(&app, HitTarget::Tab(Workspace::Network));
    app.handle_mouse(click(tab.x, tab.y));
    assert_eq!(app.ui.workspace, Workspace::Network);
}

#[test]
fn clicking_rows_selects_and_double_click_opens_detail() {
    let mut app = app();
    draw(&mut app);
    let row = find(&app, HitTarget::Row(PanelId::Processes, 2));
    app.handle_mouse(click(row.x + 1, row.y));
    assert_eq!(app.selected_process().unwrap().identity.pid, 3);
    app.handle_mouse(click(row.x + 1, row.y));
    assert_eq!(app.mode, AppMode::ProcessDetail);
}

#[test]
fn wheel_scrolls_the_panel_under_the_cursor() {
    let mut app = app();
    app.ui.set_workspace(Workspace::Network);
    draw(&mut app);
    assert_eq!(app.ui.focus, PanelId::Interfaces);
    let traffic = find(&app, HitTarget::Panel(PanelId::Traffic));
    app.handle_mouse(mouse(MouseEventKind::ScrollDown, traffic.x + 2, traffic.y + 3));
    assert_eq!(app.ui.list(PanelId::Traffic).selected, 3);
    assert_eq!(app.ui.list(PanelId::Interfaces).selected, 0);
}

#[test]
fn confirmation_modal_blocks_background_and_exposes_buttons() {
    let mut app = app();
    app.update(Action::Terminate);
    draw(&mut app);
    let tab = find(&app, HitTarget::Tab(Workspace::Disks));
    assert!(app.handle_mouse(click(tab.x, tab.y)).is_empty());
    assert_eq!(app.ui.workspace, Workspace::Overview);
    let cancel = find(&app, HitTarget::Button(Action::Cancel));
    app.handle_mouse(click(cancel.x, cancel.y));
    assert_eq!(app.mode, AppMode::Dashboard);

    app.update(Action::Terminate);
    draw(&mut app);
    let confirm = find(&app, HitTarget::Button(Action::Confirm));
    let effects = app.handle_mouse(click(confirm.x, confirm.y));
    assert!(matches!(effects.as_slice(), [Effect::SendSignal(_)]));
}

#[test]
fn footer_hints_are_buttons() {
    let mut app = app();
    draw(&mut app);
    let quit = find(&app, HitTarget::Button(Action::Quit));
    assert_eq!(app.handle_mouse(click(quit.x, quit.y)), vec![Effect::Quit]);
}
```

Run: `cargo test --test hit_tests`
Expected: FAIL — `could not find hit in ui`.

- [ ] **Step 2: Create `src/ui/hit.rs`**

```rust
use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};

use super::state::{PanelId, Workspace};
use crate::input::action::Action;

const DOUBLE_CLICK: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitTarget {
    Tab(Workspace),
    Panel(PanelId),
    Row(PanelId, usize),
    Button(Action),
    Blocker,
}

/// Clickable regions of the last frame; later entries are on top.
#[derive(Clone, Debug, Default)]
pub struct HitMap {
    entries: Vec<(Rect, HitTarget)>,
}

impl HitMap {
    pub fn push(&mut self, area: Rect, target: HitTarget) {
        if area.width > 0 && area.height > 0 {
            self.entries.push((area, target));
        }
    }

    pub fn at(&self, column: u16, row: u16) -> Option<HitTarget> {
        let point = Position::new(column, row);
        self.entries
            .iter()
            .rev()
            .find(|(area, _)| area.contains(point))
            .map(|(_, target)| *target)
    }

    pub fn panel_at(&self, column: u16, row: u16) -> Option<PanelId> {
        let point = Position::new(column, row);
        for (area, target) in self.entries.iter().rev() {
            if !area.contains(point) {
                continue;
            }
            match target {
                HitTarget::Panel(panel) | HitTarget::Row(panel, _) => return Some(*panel),
                HitTarget::Blocker => return None,
                HitTarget::Tab(_) | HitTarget::Button(_) => {}
            }
        }
        None
    }

    pub fn has_blocker(&self) -> bool {
        self.entries
            .iter()
            .any(|(_, target)| *target == HitTarget::Blocker)
    }

    pub fn entries(&self) -> &[(Rect, HitTarget)] {
        &self.entries
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ClickMemory {
    last: Option<(PanelId, usize, Instant)>,
}

pub fn mouse_action(
    hits: &HitMap,
    event: &MouseEvent,
    focus: PanelId,
    memory: &mut ClickMemory,
    now: Instant,
) -> Option<Action> {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => match hits.at(event.column, event.row)? {
            HitTarget::Tab(workspace) => Some(Action::GoWorkspace(workspace)),
            HitTarget::Panel(panel) => Some(Action::Focus(panel)),
            HitTarget::Button(action) => Some(action),
            HitTarget::Blocker => None,
            HitTarget::Row(panel, index) => {
                let double = memory.last.is_some_and(|(last_panel, last_index, at)| {
                    last_panel == panel
                        && last_index == index
                        && now.saturating_duration_since(at) <= DOUBLE_CLICK
                });
                memory.last = (!double).then_some((panel, index, now));
                Some(if double {
                    Action::Open
                } else {
                    Action::Select(panel, index)
                })
            }
        },
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            if hits.has_blocker() {
                return None;
            }
            let delta = if event.kind == MouseEventKind::ScrollUp { -3 } else { 3 };
            let panel = hits.panel_at(event.column, event.row).unwrap_or(focus);
            Some(Action::Scroll(panel, delta))
        }
        _ => None,
    }
}
```

In `src/ui/mod.rs`: add `pub mod hit;` and the field `pub hits: hit::HitMap,` to `RenderOutput`.

- [ ] **Step 3: Route mouse events in `src/app.rs`**

Add field `clicks: ClickMemory` (init `ClickMemory::default()`), import `use crate::ui::hit::{mouse_action, ClickMemory};`, and replace `handle_mouse`:

```rust
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Vec<Effect> {
        if self.mode == AppMode::Filtering {
            return Vec::new();
        }
        match mouse_action(
            &self.last_render.hits,
            &mouse,
            self.ui.focus,
            &mut self.clicks,
            Instant::now(),
        ) {
            Some(action) => self.update(action),
            None => Vec::new(),
        }
    }
```

Remove the now-unused `MouseEventKind` import.

- [ ] **Step 4: Register rows and panels**

Add to `src/ui/components/table.rs`:

```rust
use ratatui::layout::Rect;

use crate::ui::hit::HitTarget;

/// Register a bordered table panel and its visible rows (1 border + 1 header line above rows).
pub fn register_rows(cx: &mut RenderCx, panel: PanelId, area: Rect, len: usize) {
    cx.out.hits.push(area, HitTarget::Panel(panel));
    let viewport = area.height.saturating_sub(3) as usize;
    let offset = cx.app.ui.list(panel).offset.min(len.saturating_sub(1));
    for (row, index) in (offset..len).take(viewport).enumerate() {
        cx.out.hits.push(
            Rect {
                x: area.x + 1,
                y: area.y + 2 + row as u16,
                width: area.width.saturating_sub(2),
                height: 1,
            },
            HitTarget::Row(panel, index),
        );
    }
}
```

Call it right after each table render:
- `process_table`: `register_rows(cx, PanelId::Processes, area, processes.len());`
- `interface_table`: `register_rows(cx, PanelId::Interfaces, area, cx.app.state.interfaces.len());`
- `traffic_table`: `register_rows(cx, PanelId::Traffic, area, talkers.len());` (compute `let count = talkers.len();` before the table consumes the rows to satisfy the borrow checker)
- `disks`: `register_rows(cx, PanelId::Disks, area, disks.len());`

Non-table panels register themselves as whole panels: in `overview::resources` add `cx.out.hits.push(area, HitTarget::Panel(PanelId::Resources));`, in `overview::details` `…Panel(PanelId::Details)`, in `more` `…Panel(PanelId::Settings)`.

- [ ] **Step 5: Register tabs in `components/status.rs`**

Replace the tab span construction with:

```rust
    if area.height >= 3 {
        let mut x = area.x;
        let mut tabs = Vec::new();
        for (index, workspace) in Workspace::ALL.iter().enumerate() {
            let text = format!("  {} {}  ", index + 1, workspace.label());
            let width = text.chars().count() as u16;
            cx.out.hits.push(
                Rect {
                    x,
                    y: area.y + 2,
                    width: width.min(area.right().saturating_sub(x)),
                    height: 1,
                },
                HitTarget::Tab(*workspace),
            );
            x = x.saturating_add(width);
            let style = if *workspace == cx.app.ui.workspace {
                Style::default()
                    .fg(palette.accent_fg)
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_muted)
            };
            tabs.push(Span::styled(text, style));
        }
        lines.push(Line::from(tabs));
    }
```

- [ ] **Step 6: Footer buttons in `components/footer.rs`**

In the hint loop, register each hint before pushing its spans. Replace the loop body after the wrapping check with:

```rust
        let x = area.x + width as u16;
        let y = area.y + (lines.len() - 1) as u16;
        cx.out.hits.push(
            Rect {
                x,
                y,
                width: needed as u16,
                height: 1,
            },
            HitTarget::Button(binding.action),
        );
        width += needed;
```

(the `width += needed;` line moves here from above; keep the span pushes after it). Imports: `use ratatui::layout::Rect;`, `use crate::ui::hit::HitTarget;`.

- [ ] **Step 7: Modal blocker and buttons in `components/modal.rs`**

At the start of both `detail` and `confirmation` (after the early returns, before `Clear`), add `cx.out.hits.push(area, HitTarget::Blocker);`. In `confirmation`, after rendering the paragraph, register the two halves of the button line (the 6th line inside the border):

```rust
    let line = Rect {
        x: popup.x + 1,
        y: popup.y + 6,
        width: popup.width.saturating_sub(2),
        height: 1,
    };
    let half = line.width / 2;
    cx.out.hits.push(Rect { width: half, ..line }, HitTarget::Button(Action::Confirm));
    cx.out.hits.push(
        Rect {
            x: line.x + half,
            width: line.width - half,
            ..line
        },
        HitTarget::Button(Action::Cancel),
    );
```

Change the button line text to `"Y  CONFIRM              N / ESC  CANCEL"` so each label sits in its half. Imports: `use crate::input::action::Action;`, `use crate::ui::hit::HitTarget;`.

- [ ] **Step 8: Run all checks and commit**

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: PASS (7 hit tests).

Manual: `cargo run --release` → click `3 Network` tab, click a process row, double-click it (detail opens), scroll over the traffic list, click `Q QUIT` in the footer.

```bash
git add -A src tests
git commit -m "feat: add clickable tabs, rows and footer buttons"
```

---

### Task 9: Süreç trafiği — nettop akış worker'ı

Hata 5 kapanır: `nettop` sürekli açık kalır, ardışık bloklar arasındaki farktan pid bazında byte/sn üretilir.

**Files:**
- Create: `src/collectors/traffic.rs`
- Modify: `src/collectors/mod.rs` (`pub mod traffic;`), `src/collectors/runtime.rs` (`StopGuard` `pub(crate)`, `start` traffic worker'ı ekler)
- Test: `tests/traffic_tests.rs`

**Interfaces:**
- Consumes: `Control`, `CollectorRuntime::{with_workers, adopt, add_child_killer}`, `byte_rate`, `ProcessTraffic`, `TrafficRate`, `CollectorUpdate::{Traffic, Failed}`.
- Produces (`filiz::collectors::traffic`): `NettopRow { pid, bytes_in, bytes_out }`; `NettopParser::push_line(&mut self, &str) -> Option<Vec<NettopRow>>`; `TrafficTracker::update(&mut self, &[NettopRow], Instant) -> Vec<ProcessTraffic>`; `TrafficConfig { program, args, retry_delay, max_restarts }` (`Default` = `/usr/bin/nettop -P -x -L 0 -s 2`, 10 s, 3); `spawn(TrafficConfig, Sender<CollectorUpdate>) -> (Sender<Control>, JoinHandle<()>, Box<dyn FnOnce() + Send>)`.

- [ ] **Step 1: Write the failing tests**

`tests/traffic_tests.rs`:

```rust
use std::sync::mpsc;
use std::time::{Duration, Instant};

use filiz::collectors::runtime::CollectorRuntime;
use filiz::collectors::traffic::{spawn, NettopParser, NettopRow, TrafficConfig, TrafficTracker};
use filiz::state::{CollectorUpdate, Source};

const HEADER: &str = "time,,interface,state,bytes_in,bytes_out,rx_dupe";

fn row(pid: u32, bytes_in: u64, bytes_out: u64) -> NettopRow {
    NettopRow { pid, bytes_in, bytes_out }
}

#[test]
fn parser_emits_a_block_when_the_next_header_arrives() {
    let mut parser = NettopParser::default();
    assert_eq!(parser.push_line("11:00:00.1,Safari.321,,,1000,500,0"), None, "row before header");
    assert_eq!(parser.push_line(HEADER), None);
    assert_eq!(parser.push_line("11:00:00.1,Safari.321,,,1000,500,0"), None);
    assert_eq!(parser.push_line("11:00:00.1,broken line"), None);
    assert_eq!(parser.push_line(HEADER), Some(vec![row(321, 1000, 500)]));
    assert_eq!(parser.push_line(HEADER), Some(vec![]));
}

#[test]
fn parser_takes_pid_after_last_dot() {
    let mut parser = NettopParser::default();
    parser.push_line(HEADER);
    parser.push_line("11:00:00.1,com.apple.WebKit.Networking.812,,,10,20,0");
    parser.push_line("11:00:00.1,Google Chrome H.81373,,,30,40,0");
    assert_eq!(
        parser.push_line(HEADER),
        Some(vec![row(812, 10, 20), row(81373, 30, 40)])
    );
}

#[test]
fn tracker_reports_rates_from_deltas_and_skips_idle_and_reset_counters() {
    let mut tracker = TrafficTracker::default();
    let start = Instant::now();
    assert!(tracker
        .update(&[row(1, 1000, 1000), row(2, 500, 500), row(3, 900, 900)], start)
        .is_empty());
    let rates = tracker.update(
        &[row(1, 3000, 1500), row(2, 500, 500), row(3, 100, 100)],
        start + Duration::from_secs(2),
    );
    assert_eq!(rates.len(), 1, "idle pid 2 and reset pid 3 are omitted");
    assert_eq!(rates[0].pid, 1);
    assert_eq!(rates[0].rate.rx, 1000.0);
    assert_eq!(rates[0].rate.tx, 250.0);
    let after_reset = tracker.update(&[row(3, 300, 100)], start + Duration::from_secs(4));
    assert_eq!(after_reset[0].rate.rx, 100.0);
}

fn fake(script: &str, max_restarts: u32) -> TrafficConfig {
    TrafficConfig {
        program: "/bin/sh".into(),
        args: vec!["-c".into(), script.into()],
        retry_delay: Duration::from_millis(10),
        max_restarts,
    }
}

#[test]
fn worker_streams_blocks_then_reports_exit() {
    let script = format!(
        "printf '{h}\\nx,Safari.321,,,1000,0,0\\n{h}\\n'; sleep 0.1; printf 'x,Safari.321,,,5000,0,0\\n{h}\\n'",
        h = HEADER
    );
    let (tx, rx) = mpsc::channel();
    let (_control, handle, _killer) = spawn(fake(&script, 0), tx);
    let wait = Duration::from_secs(2);
    assert_eq!(rx.recv_timeout(wait).unwrap(), CollectorUpdate::Traffic(vec![]));
    match rx.recv_timeout(wait).unwrap() {
        CollectorUpdate::Traffic(rows) => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].pid, 321);
            assert!(rows[0].rate.rx > 0.0);
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        rx.recv_timeout(wait).unwrap(),
        CollectorUpdate::Failed {
            source: Source::Traffic,
            message: "nettop exited".into()
        }
    );
    handle.join().unwrap();
}

#[test]
fn missing_program_is_reported_and_retried_up_to_the_limit() {
    let config = TrafficConfig {
        program: "/nonexistent/nettop".into(),
        args: Vec::new(),
        retry_delay: Duration::from_millis(10),
        max_restarts: 2,
    };
    let (tx, rx) = mpsc::channel();
    let (_control, handle, _killer) = spawn(config, tx);
    handle.join().unwrap();
    let failures: Vec<_> = rx.try_iter().collect();
    assert_eq!(failures.len(), 3);
    assert!(matches!(
        &failures[0],
        CollectorUpdate::Failed { source: Source::Traffic, message } if message.contains("unavailable")
    ));
}

#[test]
fn runtime_drop_kills_the_long_running_child() {
    let (tx, _rx) = mpsc::channel();
    let mut runtime = CollectorRuntime::with_workers(Vec::new(), tx.clone());
    let (control, handle, killer) = spawn(fake("printf 'time,,bytes_in,bytes_out\\n'; exec sleep 30", 3), tx);
    runtime.adopt(control, handle);
    runtime.add_child_killer(killer);
    std::thread::sleep(Duration::from_millis(200));
    let started = Instant::now();
    drop(runtime);
    assert!(started.elapsed() < Duration::from_secs(2));
}
```

Run: `cargo test --test traffic_tests`
Expected: FAIL — `could not find traffic in collectors`.

- [ ] **Step 2: Expose `StopGuard` in `src/collectors/runtime.rs`**

```rust
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
```

In `run_worker`, use `let mut guard = StopGuard::new(worker.source, updates.clone());` and `guard.disarm();`.

- [ ] **Step 3: Create `src/collectors/traffic.rs`**

```rust
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
            let Some(((rx0, tx0), seconds)) = self.previous.get(&row.pid).copied().zip(elapsed) else {
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
            args: ["-P", "-x", "-L", "0", "-s", "2"].map(String::from).to_vec(),
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

fn stop_requested(control: &Receiver<Control>) -> bool {
    matches!(
        control.try_recv(),
        Ok(Control::Stop) | Err(TryRecvError::Disconnected)
    )
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
                stream(BufReader::new(stdout), &updates);
                kill(&slot);
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

fn stream(reader: impl BufRead, updates: &Sender<CollectorUpdate>) {
    let mut parser = NettopParser::default();
    let mut tracker = TrafficTracker::default();
    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        if let Some(rows) = parser.push_line(&line) {
            let traffic = tracker.update(&rows, Instant::now());
            if updates.send(CollectorUpdate::Traffic(traffic)).is_err() {
                break;
            }
        }
    }
}
```

Add `pub mod traffic;` to `src/collectors/mod.rs`.

- [ ] **Step 4: Start the traffic worker in `CollectorRuntime::start`**

Change the end of `start` from `Self::with_workers(vec![...], updates)` to:

```rust
        let mut runtime = Self::with_workers(
            vec![/* the existing System and Platform WorkerSpecs, unchanged */],
            updates.clone(),
        );
        let (control, handle, killer) =
            super::traffic::spawn(super::traffic::TrafficConfig::default(), updates);
        runtime.adopt(control, handle);
        runtime.add_child_killer(killer);
        runtime
```

(Keep the two `WorkerSpec` literals exactly as they are; only the wrapping changes.)

- [ ] **Step 5: Run tests and checks**

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: PASS (6 traffic tests).

Manual: `cargo run --release`, press `3`, open a web page. Within ~4 s the PROCESS TRAFFIC list shows the browser with non-zero DOWN; `Q` exits in under 1 s and `pgrep nettop` prints nothing afterwards.

- [ ] **Step 6: Commit**

```bash
git add src tests
git commit -m "feat: stream per-process traffic from nettop"
```

---

### Task 10: Splash, panic hook ve dokümantasyon

Hata 10 kapanır; panik anında terminal geri yüklenir; README ve CHANGELOG yeni tuşları anlatır.

**Files:**
- Create: `src/ui/splash.rs`
- Modify: `src/ui/mod.rs` (`pub mod splash;`), `src/main.rs`, `README.md` (Controls tablosu ve Network paragrafı), `CHANGELOG.md`
- Test: `tests/ui_tests.rs`

**Interfaces:**
- Produces (`filiz::ui::splash`): `for_raw_mode(text: &str) -> String`.

- [ ] **Step 1: Write the failing test**

Add to `tests/ui_tests.rs`:

```rust
#[test]
fn splash_uses_carriage_returns_in_raw_mode() {
    let text = filiz::ui::splash::for_raw_mode("a\nb\r\nc\n");
    assert_eq!(text, "a\r\nb\r\nc\r\n");
    let logo = filiz::ui::splash::for_raw_mode(include_str!("../assets/logo.ansi"));
    assert!(!logo.replace("\r\n", "").contains('\n'));
}
```

Run: `cargo test --test ui_tests splash_uses_carriage_returns_in_raw_mode`
Expected: FAIL — `could not find splash in ui`.

- [ ] **Step 2: Create `src/ui/splash.rs`**

```rust
/// Raw mode disables output post-processing, so a bare `\n` does not return the cursor.
pub fn for_raw_mode(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}
```

Add `pub mod splash;` to `src/ui/mod.rs`.

- [ ] **Step 3: Replace `src/main.rs`**

```rust
use std::io::{self, stdout, Write};
use std::time::Duration;

use crossterm::{
    cursor::{MoveTo, Show},
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use filiz::app;
use filiz::collectors::runtime::CollectorRuntime;
use filiz::ui::splash;
use ratatui::{backend::CrosstermBackend, Terminal};

const SPLASH: Duration = Duration::from_millis(650);

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout(), EnterAlternateScreen, EnableMouseCapture) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

fn restore_terminal() {
    let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen, Show);
    let _ = disable_raw_mode();
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Restore the terminal before the default hook prints, so the message stays visible.
/// Collector threads report their own panics as `Stopped`; the UI keeps running for those.
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().name() == Some("main") {
            restore_terminal();
        }
        default(info);
    }));
}

fn main() -> anyhow::Result<()> {
    install_panic_hook();
    let _guard = TerminalGuard::enter()?;
    let (updates_tx, updates_rx) = std::sync::mpsc::channel();
    let runtime = CollectorRuntime::start(updates_tx);
    let logo = format!(
        "{}\n  for betül, with love ♡\n",
        include_str!("../assets/logo.ansi")
    );
    print!("{}", splash::for_raw_mode(&logo));
    stdout().flush()?;
    if event::poll(SPLASH)? {
        let _ = event::read()?;
    }
    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = app::App::new(Duration::from_secs(2));
    app::run(&mut terminal, &mut app, runtime, updates_rx)
}
```


- [ ] **Step 4: Update `README.md`**

Replace the Controls table with:

```markdown
| Key | Action |
| --- | --- |
| `1`–`5`, `←` / `→` | Switch Overview, Processes, Network, Disks, More |
| `Tab` / `Shift+Tab` | Move focus between the panels of the current workspace |
| `↑` / `↓`, `PageUp` / `PageDown`, `Home` / `End` | Move in the focused list |
| `Enter` | Open process detail (process list) |
| `F` | Filter processes |
| `S` | Cycle sort: CPU → memory |
| `K` / `Shift+K` | Request terminate / kill confirmation |
| `Y` / `N` / `Esc` | Confirm / cancel a pending process action |
| `M` | Open or close the menu |
| `H` | Hide the focused panel; press again to restore |
| `L` | Cycle compact, balanced and spacious layout |
| `T` | Cycle Forest, Amber, Mono and Solarized themes |
| `R` | Refresh now |
| `Q` / `Ctrl+C` | Quit |
| Mouse | Click tabs, rows and footer hints; double-click a process for detail; wheel scrolls the panel under the cursor |

The footer always lists the keys available in the current context.
```

Replace the paragraph that starts with "The Network workspace (`3`)" with:

```markdown
The Network workspace (`3`) shows download/upload rates per interface and a
live per-process traffic list collected from `nettop`. Metrics are collected on
background threads, so the interface stays responsive while macOS tools run.
```

- [ ] **Step 5: Update `CHANGELOG.md`**

Replace the `## Unreleased` section body with:

```markdown
- Collect metrics on background threads; the interface no longer freezes during refresh.
- Show live per-process network traffic from `nettop`.
- Separate download and upload history per interface.
- Give the Processes workspace its own layout.
- Make tabs, rows and footer hints clickable; the mouse wheel scrolls the panel under the cursor.
- Sort processes with `S`; `M` now only opens the menu.
- Show context-aware key hints in the footer.
- Hide macOS system volumes from the Disks list and stop warning about missing sensors.
- Fix the startup logo rendering in raw mode and restore the terminal on panic.
```

- [ ] **Step 6: Run the full verification**

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Expected: all pass.

Manual checklist (`./target/release/filiz` in Terminal.app or iTerm2):
- Splash logo lines are aligned (no staircase); any key skips it.
- Holding `↓` never stalls; `Q` exits in under 1 s; `pgrep nettop` is empty afterwards.
- `2` shows the full process table; `3` shows moving DOWN/UP values and a process traffic list while a download runs.
- Clicking tabs/rows/footer hints works; the confirm modal ignores clicks outside its buttons.
- Status band stays `SYSTEM NORMAL` on a Mac without a temperature sensor.

- [ ] **Step 7: Commit**

```bash
git add src tests README.md CHANGELOG.md
git commit -m "fix: render splash in raw mode and restore terminal on panic"
```

---

## Deferred to later sub-projects

- `App::animating()` tick: no animation consumer exists yet; the installer/transition sub-project adds it together with its first user.
- Command palette content behind `M`, theme picker, accent colour and persistent config.
- Release pipeline and the redesigned installer download screen.
