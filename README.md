# Tilo

[![Build](https://github.com/pchomik/tilo/actions/workflows/build.yml/badge.svg)](https://github.com/pchomik/tilo/actions/workflows/rust.yml)
[![Windows 10](https://img.shields.io/badge/Windows-10-0078D6?logo=windows)](https://www.microsoft.com/windows)
[![Windows 11](https://img.shields.io/badge/Windows-11-0078D6?logo=windows)](https://www.microsoft.com/windows)

Tilo is a tiling window manager for Windows with virtual desktop support.
It arranges windows into grid-based layouts (Monocle, Columns, Rows, Grid),
tracks them across monitors and virtual desktops, and re-tiles automatically
when windows are opened, closed, minimized, or restored.

## How it works

Tilo places a grid of cells (rows × columns) over each monitor's work area.
A layout selects which cells form each *area*, and Tilo converts those cells
into pixel rectangles with the configured gap.

- New windows take the first area; existing windows shift toward later areas.
- Minimized windows are untracked; remaining windows shift back.
- The last area is stacked: extra windows share its rectangle. Tilo never
  hides, activates, or reorders stacked windows.
- Window positions are verified periodically and corrected if they drift.
- Window movement never changes the z-order (`SWP_NOZORDER`).
- The focused window can be highlighted with a configurable colored border.

## Requirements

- Windows 10 or Windows 11
- No runtime dependencies; both executables are standalone
- [PowerToys](https://learn.microsoft.com/en-us/windows/powertoys/) (optional,
  for keyboard shortcuts via Keyboard Manager)

## Installation

### Binaries

Pre-built binaries are available as ZIP archives from two places:

- **Releases** — download the attached ZIP files from the latest
  [release tag](https://github.com/pchomik/tilo/releases).
- **Actions** — download build artifacts from the
  [Actions](https://github.com/pchomik/tilo/actions) tab for any
  branch or pull request.

Each archive contains `tilosrv.exe` and `tiloctl.exe` for one Windows version
(`windows10` or `windows11`). Extract both executables to a directory of your
choice, for example `C:\Apps\tilo\`.

Run `tilosrv.exe` to start the tiling service. It runs in the background with
a system tray icon.

### Windows security warning

Because the binaries are not code-signed, Windows SmartScreen shows an
"Unknown publisher" warning the first time you run each executable. This is
expected. Click **More info** and then **Run anyway** to confirm the exception.
You only need to do this once per executable.

## Configuration

Copy [example/config.toml](example/config.toml) to:

```
%USERPROFILE%\.config\tilo\config.toml
```

### Reference

The full configuration with every option and its default, explained inline:

```toml
# Order of layouts used by the `cycle-layout` command.
# Valid names: Monocle, Columns, Rows, Grid.
cycle_order = ["Monocle", "Columns", "Rows", "Grid"]

# Global grid laid over each monitor's work area. Layouts select cells from
# this grid; Tilo converts them into pixel rectangles.
[grid]
rows = 4     # grid rows per monitor (default: 4)
columns = 4  # grid columns per monitor (default: 4)
gap = 8      # pixels between cells and around the work area (default: 0)

# Periodic full window scan; complements the WinEvent hook.
[scan]
enabled = true      # enable the periodic scan (default: true)
interval_ms = 1000  # scan interval in milliseconds (default: 1000)

# Periodic verification of window positions.
[periodic_check]
enabled = true      # enable position verification (default: true)
interval_ms = 1000  # check interval in milliseconds (default: 1000)
tolerance = 5       # allowed pixel deviation before correcting (default: 5)

# Windows matching ANY specified field are excluded from tiling. Repeat this
# table for each rule. Both fields are regular expressions.
[[ignore]]
process = "explorer.exe"  # regex matched against the process name

[[ignore]]
title = "Settings"        # regex matched against the window title

# Colored frame drawn around the currently focused window.
[window_border]
enabled = true       # draw the active-window border (default: true)
width = 3            # border thickness in logical pixels, DPI-scaled (default: 3)
radius = 0           # corner radius in logical pixels, 0 = sharp corners (default: 0)
color = "#8dbcff"    # hex RGB border color (default: "#8dbcff")

# Windows matching ANY specified field (regex) never get a border. Repeat this
# table for each rule. No rules are defined by default.
# [[window_border.ignore]]
# process = "explorer.exe"  # regex matched against the process name
#
# [[window_border.ignore]]
# title = "System"          # regex matched against the window title

# One entry per virtual desktop. Repeat for each desktop.
[[virtual_desktops]]
name = "1"  # display name of the desktop

  # One entry per monitor, ordered left to right. Repeat for each monitor.
  [[virtual_desktops.monitors]]
  # Optional monitor device name. When omitted, monitors are matched by
  # position, left to right.
  # monitor = '\\.\DISPLAY1'
  # Layout for this monitor. One of:
  #   Monocle - single work-area-sized stack; Windows controls z-order
  #   Columns - up to max_columns vertical areas; extra windows stack in the last column
  #   Rows    - up to max_rows horizontal areas; extra windows stack in the last row
  #   Grid    - one area per grid cell in row-major order; extra windows stack in the last cell
  layout = "Columns"
  max_columns = 3  # maximum column areas (Columns layout)
  # max_rows = 2   # maximum row areas (Rows layout)

[[virtual_desktops]]
name = "2"
  [[virtual_desktops.monitors]]
  layout = "Rows"
  max_rows = 2
  [[virtual_desktops.monitors]]
  layout = "Monocle"
```

## Keyboard shortcuts

The recommended way to bind keys is
[PowerToys Keyboard Manager](https://learn.microsoft.com/en-us/windows/powertoys/keyboard-manager#remap-a-shortcut-to-start-an-app),
which runs `tiloctl.exe` with a subcommand.

Use these settings when remapping a shortcut to an app:

| Option     | Value                        |
| ---------- | ---------------------------- |
| App        | Full path to `tiloctl.exe`   |
| Args       | Subcommand and its arguments |
| Start in   | Default value                |
| Elevation  | Normal                       |
| If running | Do nothing                   |
| Visibility | Hidden                       |

Example binding for moving a window right:

| Option     | Value                      |
| ---------- | -------------------------- |
| App        | `C:\Apps\tilo\tiloctl.exe` |
| Args       | `move-right`               |
| Start in   | Default value              |
| Elevation  | Normal                     |
| If running | Do nothing                 |
| Visibility | Hidden                     |

## Command-line client

`tiloctl.exe` sends commands to `tilosrv.exe` over the named pipe
`\\.\pipe\tilosrv-pipe`. The payload is JSON:

```json
{
    "command": "MoveWindowRight",
    "args": []
}
```

Virtual desktop indexes start at 0.

| Subcommand                       | Args            | Description                                                                                          |
| -------------------------------- | --------------- | ---------------------------------------------------------------------------------------------------- |
| `move-to-next-virtual-desktop`   | —               | Move active window to next virtual desktop                                                           |
| `move-to-prev-virtual-desktop`   | —               | Move active window to previous virtual desktop                                                       |
| `move-to-virtual-desktop`        | `--idx N`       | Move active window to virtual desktop N                                                              |
| `switch-to-next-virtual-desktop` | —               | Switch to next virtual desktop                                                                       |
| `switch-to-prev-virtual-desktop` | —               | Switch to previous virtual desktop                                                                   |
| `switch-to-virtual-desktop`      | `--idx N`       | Switch to virtual desktop N                                                                          |
| `retile-monitor`                 | —               | Recompute and apply tiling for the active monitor                                                    |
| `retile-vd`                      | —               | Recompute and apply tiling for all monitors on the current desktop                                   |
| `move-right`                     | —               | Move right in Columns/Grid; crosses to next monitor at the edge; changes monitor in Monocle/Rows     |
| `move-left`                      | —               | Move left in Columns/Grid; crosses to previous monitor at the edge; changes monitor in Monocle/Rows  |
| `move-up`                        | —               | Move up in Rows/Grid                                                                                 |
| `move-down`                      | —               | Move down in Rows/Grid                                                                               |
| `cycle-layout`                   | —               | Cycle the layout of the monitor with the focused window using `cycle_order`                          |
| `set-layout`                     | `--layout NAME` | Set the layout of the monitor with the focused window (`Monocle`, `Columns`, `Rows`, or `Grid`, case-insensitive) |

## Building

Requires the Rust toolchain (edition 2024).

```sh
# Windows 10 (default)
cargo build --release

# Windows 11
cargo build --release --no-default-features --features windows11
```

Binaries are written to `target/release/`.

## Limitations

### Visual blink on desktop switch

The screen may blink when switching desktops via `tiloctl`. Use the built-in
Windows shortcuts for desktop switching instead. Tilo still provides this
functionality if needed.

### Windows 10 virtual desktop offset

On Windows 10, create one extra virtual desktop. The underlying library does
not return the last desktop.

### Monitor arrangement

Cross-monitor window movement only works with horizontally arranged monitors
(left/right). Vertically stacked monitors are not supported.

## License

Copyright © Pawel Chomicki. Licensed under the
[GNU General Public License v3.0](LICENSE).
