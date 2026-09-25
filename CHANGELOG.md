# Changelog

Changes to Filiz are recorded here.

## 0.1.0 - 2026-09-24

- Add the live Ratatui dashboard for macOS.
- Add CPU, memory, disk, network, process, battery, and temperature metrics.
- Add filtering, sorting, process detail, and confirmed process actions.
- Add CI and release-oriented development checks.

## Unreleased

- Collect metrics on background threads; the interface no longer freezes during refresh.
- Show live per-process network traffic from `nettop`.
- Separate download and upload history per interface.
- Give the Processes workspace its own layout.
- Make tabs, rows and footer hints clickable; the mouse wheel scrolls the panel under the cursor.
- Sort processes with `S`; `M` now only opens the menu.
- Show context-aware key hints in the footer.
- Hide macOS system volumes from the Disks list and stop warning about missing sensors.
- Fix the startup logo rendering in raw mode and restore the terminal on panic.
