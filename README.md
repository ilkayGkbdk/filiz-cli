# Filiz

<p align="center">
  <img src="assets/logo-256.png" alt="filiz logo" width="128">
</p>

Filiz is a live macOS system monitor for the terminal. It combines a readable
resource dashboard with a process table and explicit confirmation for process
actions.

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

## What it shows

- CPU usage, core count, and live history
- Memory and disk usage
- Network receive/transmit rates
- Battery, uptime, and temperature when available
- Processes sorted by CPU or memory
- Process detail and text filtering
- Safe terminate/kill flow with explicit `Y` confirmation

## Controls

| Key | Action |
| --- | --- |
| `Tab` | Change focused panel |
| `↑` / `↓` | Select a process |
| `Enter` | Open process detail |
| `F` | Filter processes |
| `C` / `M` | Sort by CPU / memory |
| `K` / `Shift+K` | Request terminate / kill confirmation |
| `Y` | Confirm a pending process action |
| `N` / `Esc` | Cancel or close |
| `Q` | Quit |

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
