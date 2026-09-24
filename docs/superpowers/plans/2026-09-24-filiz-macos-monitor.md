# Filiz macOS Monitor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and publish the first private GitHub-ready macOS release of Filiz: a Rust/Ratatui live system monitor with readable dashboard panels and safe process actions.

**Architecture:** Independent collectors produce a shared `SystemSnapshot`; an application state machine handles input, selection, filtering, confirmation, and errors; Ratatui renders only the latest snapshot. macOS-specific metrics are isolated behind platform modules, while documentation and CI make the private repository installable and maintainable.

**Tech Stack:** Rust stable, Ratatui, Crossterm, sysinfo, macOS native APIs where needed, Cargo, GitHub Actions, MIT License.

**Spec:** `docs/superpowers/specs/2026-09-24-filiz-macos-monitor-design.md`

## Global Constraints

- Target only macOS for the first release.
- The command must run as `filiz` after a Cargo install or release-binary install.
- A single collector failure must not stop the dashboard; unavailable values render as `N/A`.
- Destructive process actions require an explicit confirmation view showing process name and PID.
- UI uses a dark olive/black palette with green normal, yellow warning, and red danger semantics.
- Raw mode and alternate screen must always be restored on normal and error exits.
- Required checks are `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- Keep the repository private; update GitHub metadata without changing visibility.

## Planned File Structure

- `Cargo.toml`: package metadata, binary name, dependencies, and macOS target configuration.
- `src/main.rs`: process startup, terminal lifecycle, and top-level error reporting.
- `src/app.rs`: event loop, application state, keyboard handling, refresh cadence, and action dispatch.
- `src/model.rs`: snapshot, resource, process, event, and collector error types.
- `src/collectors/mod.rs`: collector orchestration and snapshot assembly.
- `src/collectors/system.rs`: CPU, memory, disk, and network collection through `sysinfo`.
- `src/collectors/processes.rs`: process snapshots and process metadata.
- `src/collectors/macos.rs`: battery and temperature adapters with graceful unavailable results.
- `src/actions.rs`: process termination service with typed errors.
- `src/ui/mod.rs`: top-level layout and panel composition.
- `src/ui/widgets.rs`: reusable resource cards, process table, detail panels, footer, and modal rendering.
- `src/ui/theme.rs`: Filiz color and spacing constants.
- `tests/model_tests.rs`: snapshot, filtering, sorting, and action-state tests.
- `.github/workflows/ci.yml`: format, clippy, test, and macOS build checks.
- `README.md`, `LICENSE`, `CONTRIBUTING.md`, `CHANGELOG.md`: public-facing repository documentation.
- `.gitignore`: Rust, macOS, editor, and local brainstorm artifacts.

### Task 1: Scaffold the Rust binary and repository hygiene

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`
- Create: `README.md`
- Create: `LICENSE`
- Create: `CONTRIBUTING.md`
- Create: `CHANGELOG.md`
- Test: Cargo metadata and empty binary build

**Interfaces:**
- Produces a binary named `filiz` and a compiling `main()` entry point.

- [ ] **Step 1: Write the package manifest and binary entry point.** Set package name `filiz`, edition `2021`, and dependencies `anyhow`, `crossterm`, `ratatui`, and `sysinfo`; make `main` return `anyhow::Result<()>`.
- [ ] **Step 2: Add repository hygiene and MIT documentation.** Ignore `target/`, `.DS_Store`, `.superpowers/`, IDE files, and local environment files; add the full MIT text with copyright holder `ilkayGkbdk`; document the project as an early private macOS monitor and mark the design preview as non-runtime artwork.
- [ ] **Step 3: Run the scaffold checks.** Run `cargo check` and `cargo test`; expected result is a successful build with zero tests failing.
- [ ] **Step 4: Commit the scaffold.** Commit `feat: scaffold Filiz Rust binary`.

### Task 2: Define snapshot models and deterministic pure logic

**Files:**
- Create: `src/model.rs`
- Create: `tests/model_tests.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces `SystemSnapshot`, `ResourceMetric`, `ProcessInfo`, `SystemEvent`, `CollectorWarning`, `SortMode`, and `AppMode`.
- Produces `filter_processes(processes: &[ProcessInfo], query: &str) -> Vec<ProcessInfo>`.
- Produces `sort_processes(processes: &mut [ProcessInfo], mode: SortMode)`.

- [ ] **Step 1: Write failing tests for filtering and sorting.** Use fixed `ProcessInfo` values and assert case-insensitive name/command filtering plus CPU and memory descending order.
- [ ] **Step 2: Run `cargo test model_tests` and verify the tests fail** because the model types and functions do not exist.
- [ ] **Step 3: Implement the model types and pure functions.** Store unavailable metrics as `Option<T>` or typed warning values; keep all sorting deterministic by using PID as a tie-breaker.
- [ ] **Step 4: Run `cargo test model_tests` and verify it passes.** Also run `cargo fmt --check`.
- [ ] **Step 5: Commit `test: define Filiz snapshot and process logic`**.

### Task 3: Implement system and process collectors

**Files:**
- Create: `src/collectors/mod.rs`
- Create: `src/collectors/system.rs`
- Create: `src/collectors/processes.rs`
- Modify: `src/model.rs`
- Modify: `src/main.rs`

**Interfaces:**
- `pub trait Collector { fn collect(&mut self) -> CollectorResult; }`.
- `pub struct CollectorSet` with `fn snapshot(&mut self) -> SystemSnapshot`.
- `SystemCollector::new() -> Self` and `ProcessCollector::new() -> Self`.

- [ ] **Step 1: Add collector result types and a `Collector` trait.** Each collector returns its own partial result plus warnings instead of propagating a missing metric as a fatal error.
- [ ] **Step 2: Write unit tests for byte-rate and percentage conversion helpers** using two fixed samples and assert the expected per-second values.
- [ ] **Step 3: Implement `SystemCollector` with `sysinfo`.** Refresh CPU, memory, disks, networks, and host uptime; derive percentages and rates from previous samples; preserve `N/A` for empty or unsupported values.
- [ ] **Step 4: Implement `ProcessCollector`.** Refresh process data, map PID/name/command/CPU/memory/user/status into `ProcessInfo`, and return a stable list for the UI.
- [ ] **Step 5: Assemble `CollectorSet::snapshot`.** Collect all sections independently, merge warnings, and always return a usable snapshot.
- [ ] **Step 6: Run `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check`.** Expected result is PASS on the macOS development machine.
- [ ] **Step 7: Commit `feat: add system and process collectors`**.

### Task 4: Add macOS battery, temperature, and process action services

**Files:**
- Create: `src/collectors/macos.rs`
- Create: `src/actions.rs`
- Modify: `src/collectors/mod.rs`
- Modify: `src/model.rs`
- Create: `tests/actions_tests.rs`

**Interfaces:**
- `MacOsCollector::new() -> Self` and `fn collect(&mut self) -> MacOsMetrics`.
- `ProcessAction::terminate(pid: u32) -> Result<(), ActionError>`.
- `ProcessAction::kill(pid: u32) -> Result<(), ActionError>`.
- `ActionError` variants distinguish permission denied, missing process, and OS failure.

- [ ] **Step 1: Write failing action tests** for confirmation-state transitions and typed error-to-user-message mapping; do not send real signals in unit tests.
- [ ] **Step 2: Implement macOS metric adapters.** Use safe command/API boundaries for battery and temperature; parse output into typed values; return a warning and `None` when macOS does not expose a field.
- [ ] **Step 3: Implement process actions.** Resolve the selected PID at action time, send terminate or kill through a small service, and map OS errors without panicking.
- [ ] **Step 4: Run unit tests and a manual permission/error check** with a harmless user-owned process; verify root-owned processes produce a readable error.
- [ ] **Step 5: Run the full quality checks and commit `feat: add macOS metrics and safe process actions`**.

### Task 5: Build the Ratatui dashboard and terminal lifecycle

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/widgets.rs`
- Create: `src/ui/theme.rs`
- Modify: `src/app.rs`
- Modify: `src/main.rs`

**Interfaces:**
- `pub fn render(frame: &mut Frame, app: &App)`.
- `App::new(refresh: Duration) -> Self`.
- `App::handle_key(key: KeyEvent) -> AppCommand`.
- `AppCommand` variants cover `Quit`, `Refresh`, `OpenProcess`, `BeginAction`, `ConfirmAction`, `CancelAction`, and `Noop`.

- [ ] **Step 1: Write app-state tests** for `Q`, `Tab`, arrow navigation, `Enter`, `K`, `Esc`, and confirmation acceptance/rejection.
- [ ] **Step 2: Implement `App` and the event loop.** Use a 2-second refresh tick, non-blocking key polling, and snapshot replacement; keep collection work separate from drawing.
- [ ] **Step 3: Implement the top-level layout.** Match the approved visual direction: status strip, resource grid, process table, detail row, and keyboard footer.
- [ ] **Step 4: Implement reusable widgets.** Use large readable values, concise labels, sparklines, warning colors, `N/A` states, selected rows, detail view, and action confirmation modal.
- [ ] **Step 5: Wire terminal setup and cleanup.** Enter alternate screen/raw mode before the loop and restore both in a drop guard or guaranteed cleanup path.
- [ ] **Step 6: Run the binary manually.** Verify the dashboard, refresh, resize behavior, navigation, filter mode, process detail, confirmation cancellation, and clean `Q` exit.
- [ ] **Step 7: Commit `feat: add live Ratatui dashboard`**.

### Task 6: Add tests, CI, and release-oriented command behavior

**Files:**
- Modify: `tests/model_tests.rs`
- Create: `tests/ui_tests.rs`
- Create: `.github/workflows/ci.yml`
- Modify: `README.md`
- Modify: `Cargo.toml`

**Interfaces:**
- CI runs the same commands developers run locally: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and a macOS target build.

- [ ] **Step 1: Add render smoke tests** that render the dashboard into fixed terminal buffers at normal and narrow sizes and assert that `Sistem normal`, `CPU`, `BELLEK`, and `Süreçler` remain present.
- [ ] **Step 2: Add the GitHub Actions workflow.** Use macOS latest, install stable Rust, cache Cargo, run formatting, clippy, tests, and `cargo build --release`.
- [ ] **Step 3: Complete README installation instructions.** Include `cargo install --git https://github.com/ilkayGkbdk/filiz-cli.git`, binary usage, macOS requirement, controls, feature scope, development commands, design preview, license, and private-repo note.
- [ ] **Step 4: Run all local checks** and confirm CI YAML parses as valid YAML.
- [ ] **Step 5: Commit `ci: add quality checks and installation docs`**.

### Task 7: Configure and push the private GitHub repository

**Files:**
- Modify: GitHub repository metadata through `gh`
- Modify: `README.md` if repository links or badges need final values
- Modify: `CHANGELOG.md`

**Interfaces:**
- Remote remains `https://github.com/ilkayGkbdk/filiz-cli.git`.
- Repository visibility remains private.

- [ ] **Step 1: Verify GitHub auth and remote ownership** with `gh auth status`, `git remote -v`, and `gh repo view ilkayGkbdk/filiz-cli`.
- [ ] **Step 2: Set the repository description and topics** to describe a macOS terminal system monitor, keeping the repository private.
- [ ] **Step 3: Update `CHANGELOG.md` with the initial `0.1.0` entry** and the supported feature set.
- [ ] **Step 4: Run the full verification suite** immediately before pushing.
- [ ] **Step 5: Push `main` and verify the remote commit** with `git push -u origin main` and `gh run list`.

## Verification Checklist

Before claiming completion, verify:

- `cargo fmt --check` passes.
- `cargo clippy -- -D warnings` passes.
- `cargo test` passes.
- `cargo build --release` passes on macOS.
- `filiz` opens and exits cleanly.
- Terminal resize does not panic.
- Missing battery/temperature data displays `N/A`.
- Process confirmation cannot kill without explicit confirmation.
- README installation command matches the repository URL.
- GitHub repository remains private and contains the pushed `main` branch.
