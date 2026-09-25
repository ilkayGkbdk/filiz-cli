# Filiz

<p align="center">
  <img src="assets/logo-256.png" alt="filiz logo" width="128">
</p>

Filiz is a live macOS system monitor for the terminal. It combines a readable
resource dashboard, workspace-based navigation, network/download visibility,
and explicit confirmation for process actions.

The first release targets macOS only and is currently developed in a private
repository.

## Brand assets

The source logo and terminal-sized PNG variants live in [assets](assets/). PNG
files are used for README, release, and installer surfaces. The runtime
terminal banner will use an ANSI/Unicode conversion so it remains compatible
with ordinary terminals.

The startup splash uses `assets/logo.ansi`, generated from the master PNG with
Chafa. To regenerate it after changing the logo:

```sh
brew install chafa
chafa -f symbols -c 256 -s 32x12 assets/logo.png > assets/logo.ansi
```

## Install

Recommended one-command installer with a fullscreen terminal progress screen:

```sh
curl -fsSL https://raw.githubusercontent.com/ilkayGkbdk/filiz-cli/main/scripts/install.sh | bash
```

With Rust installed:

```sh
cargo install --git https://github.com/ilkayGkbdk/filiz-cli.git
filiz
```

To uninstall Filiz:

```sh
./scripts/uninstall.sh
```

For a Cargo-only installation, the direct command is also available:

```sh
cargo uninstall filiz
```

Filiz requires a recent stable Rust toolchain and a macOS terminal with support
for alternate-screen and raw-mode input. Battery and temperature fields may
show `N/A` when macOS does not expose them on a particular machine.

The installer checks Git and Cargo, installs the latest `main` revision, and
verifies the `filiz` command.

## What it shows

- CPU usage, idle percentage, core count, and live history
- Memory used, free, available, and total capacity
- Disk used, free, total capacity, and usage thresholds
- Network/download and upload rates, session totals, peaks, and interfaces
- Battery, uptime, and temperature when available
- Processes sorted by CPU or memory
- Process detail and text filtering
- Safe terminate/kill flow with explicit `Y` confirmation

## Controls

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

The Network workspace (`3`) shows download/upload rates per interface and a
live per-process traffic list collected from `nettop`. Metrics are collected on
background threads, so the interface stays responsive while macOS tools run.

Every refresh is read-only until a process action is explicitly confirmed.

## Design preview

The dashboard follows a dark olive/black terminal design: status first, large
resource values next, then processes and details. The visual mockup used during
design exploration is a design reference, not a runtime screenshot.

## Development

Install the stable Rust toolchain, then run:

```sh
cargo check
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

The live app can be started with:

```sh
cargo run --release
```

## Scope

Filiz currently focuses on local macOS monitoring. Linux/Windows support,
remote monitoring, persistent history, disk cleanup, and system-service
management are intentionally outside the first release.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the local checks and commit workflow.

## License

Filiz is licensed under the MIT License. See [LICENSE](LICENSE).
