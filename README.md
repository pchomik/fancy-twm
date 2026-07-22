# FancyTWM

FancyTWM is a tiling window manager for Windows with first-class virtual
desktop support. It arranges your windows into configurable grid-based layouts
(Monocle, Columns, Rows), tracks windows across monitors and virtual desktops,
and keeps everything in place automatically.

## Motivation

Windows' built-in tiling and FancyZones are useful, but they lack automatic
window management across virtual desktops and monitors. FancyTWM fills this gap:
it watches your windows, assigns them to layout areas, and re-tiles them as you
open, close, minimize, or move them — all while preserving window z-order.

## How it works

FancyTWM reads a **grid** definition (rows × columns) from its configuration and
lays it over each monitor's work area. A **layout** (Monocle, Columns, or Rows)
selects which grid cells belong to each *area*. A separate position calculator
turns those cells into concrete pixel rectangles, applying the configured gap.

- **New windows** take the first area; existing windows shift toward later areas.
  Windows already in the last area stay put.
- **Minimized windows** are treated as untracked — remaining windows shift back
  toward the first area.
- **The last area is tabbed**: when more windows occupy it than there are areas,
  only one is visible and you cycle through them with the stack commands.
- Window positions are verified periodically and corrected if they drift.
- Window movement never changes the z-order (`SWP_NOZORDER`).

## Installation and Configuration

### Configuration

Copy the example configuration file [config.toml](example/config.toml) to
`%APPDATA%\FancyTWM\config.toml` and customize it to suit your preferences.

### Binaries

Pre-built binaries are available through GitHub Actions.
Download the latest ZIP archive, extract it, and place `fancytwm.exe` and
`fancyctl.exe` in a directory of your choice.

Execute both applications before proceeding, as Windows will display a warning
about the unknown publisher. This warning appears because the binaries are not
signed and the application has low usage. To use them properly, confirm the
exception when prompted.

### Key Bindings

The recommended approach is to configure keys via
[Keyboard Manager](https://learn.microsoft.com/en-us/windows/powertoys/keyboard-manager#remap-a-shortcut-to-start-an-app),
which invokes `fancyctl.exe` via the command line to trigger actions.
These are the parameters which can be configured:

| Option     | Value                              |
| ---------- | ---------------------------------- |
| App        | Path to `fancyctl.exe` application |
| Args       | Command and all arguments          |
| Start in   | Default value                      |
| Elevation  | Normal                             |
| If running | Do nothing                         |
| Visibility | Hidden                             |

#### Example

| Option     | Value                  |
| ---------- | ---------------------- |
| App        | `C:\Apps\fancyctl.exe` |
| Args       | `move-right`           |
| Start in   | Default value          |
| Elevation  | Normal                 |
| If running | Do nothing             |
| Visibility | Hidden                 |

## FancyTWM client

The `fancyctl.exe` client communicates with `fancytwm.exe` via a named pipe at
`\\.\pipe\fancytwm-pipe`. Communication is a JSON payload of the form:

```json
{
    "command": "string",
    "args": ["string"]
}
```

### Commands

#### Virtual desktops

| Command                      | Args                            | Description                                     |
| ---------------------------- | ------------------------------- | ----------------------------------------------- |
| `MoveToNextVirtualDesktop`   | -                               | Move active window to next virtual desktop      |
| `MoveToPrevVirtualDesktop`   | -                               | Move active window to previous virtual desktop  |
| `MoveToVirtualDesktop`       | `[index]`                       | Move active window to specified virtual desktop |
| `SwitchToNextVirtualDesktop` | -                               | Switch to next virtual desktop                  |
| `SwitchToPrevVirtualDesktop` | -                               | Switch to previous virtual desktop              |
| `SwitchToVirtualDesktop`     | `[index]`                       | Switch to specified virtual desktop             |

**Important**: Virtual desktops are enumerated from 0.

#### Tiling

| Command               | Args | Description                                                        |
| --------------------- | ---- | ------------------------------------------------------------------ |
| `RetileActiveMonitor` | -    | Recompute and apply tiling for the active monitor                  |
| `RetileVirtualDesktop`| -    | Recompute and apply tiling for all monitors on the current desktop |

#### Window movement

| Command           | Args | Description                                                        |
| ----------------- | ---- | ------------------------------------------------------------------ |
| `MoveWindowRight` | -    | Move the focused window one area right (crosses monitors)          |
| `MoveWindowLeft`  | -    | Move the focused window one area left (crosses monitors)           |
| `MoveWindowUp`    | -    | Move the focused window one area up (Rows layout only)             |
| `MoveWindowDown`  | -    | Move the focused window one area down (Rows layout only)           |

#### Tabbed stacking

| Command          | Args | Description                                            |
| ---------------- | ---- | ------------------------------------------------------ |
| `CycleStackNext` | -    | Show the next window in the last-area tab stack        |
| `CycleStackPrev` | -    | Show the previous window in the last-area tab stack    |

### `fancyctl` subcommands

The CLI exposes the commands above as subcommands:

```
fancyctl move-to-next-virtual-desktop
fancyctl move-to-prev-virtual-desktop
fancyctl move-to-virtual-desktop --idx 0
fancyctl switch-to-next-virtual-desktop
fancyctl switch-to-prev-virtual-desktop
fancyctl switch-to-virtual-desktop --idx 0
fancyctl retile-monitor
fancyctl retile-vd
fancyctl move-right
fancyctl move-left
fancyctl move-up
fancyctl move-down
fancyctl stack-next
fancyctl stack-prev
```

## Features

### Layouts

| Layout      | Description                                                                                     |
| ----------- | ----------------------------------------------------------------------------------------------- |
| **Monocle** | A single area covering the whole grid; all windows are tabbed, only the focused one is visible  |
| **Columns** | Up to `max_columns` vertical areas; extra windows tab in the last column                        |
| **Rows**    | Up to `max_rows` horizontal areas; extra windows tab in the last row                            |

### Window tracking

Windows are tracked via WinEvent hooks (create, destroy, minimize, restore,
move/resize end), with an optional periodic scan as a fallback. Only windows
that are visible, not minimized, top-level, not tool windows, and resizable are
tiled. Windows matching any ignore rule are excluded.

### Ignore rules

Each rule may specify a `process` and/or `title` regular expression. A window is
ignored when **any** specified field matches.

## Configuration reference

```toml
[grid]
rows = 4        # grid rows per monitor
columns = 4     # grid columns per monitor
gap = 8         # pixels between cells and around the work area

[scan]
enabled = true      # enable periodic window scan fallback
interval_ms = 1000  # scan interval

[periodic_check]
enabled = true      # enable periodic position verification
interval_ms = 1000  # check interval
tolerance = 5       # allowed pixel deviation before correcting

[[ignore]]
process = "explorer.exe"   # regex on process name
title = "Settings"         # regex on window title

[[virtual_desktops]]
name = "1"
  [[virtual_desktops.monitors]]   # one entry per monitor, left to right
  layout = "Columns"              # Monocle | Columns | Rows
  max_columns = 3                 # for Columns
  max_rows = 2                    # for Rows
```

## Project Structure

| Workspace      | Purpose                                                                    |
| -------------- | -------------------------------------------------------------------------- |
| **fancy-twm**  | Core application for managing windows across monitors and virtual desktops |
| **fancy-ctl**  | Command-line tool for triggering actions                                   |
| **fancy-core** | Shared library containing code common to all FancyTWM components           |

### Module overview (`fancy-twm`)

| Module       | Responsibility                                                        |
| ------------ | --------------------------------------------------------------------- |
| `platform`   | Win10/Win11 abstraction: windows, monitors, virtual desktops, hooks   |
| `vd`         | Virtual desktop navigation operations                                 |
| `tracker`    | Window tracking (WinEvent hooks + periodic scan) and ignore rules     |
| `grid`       | Grid model and single-cell geometry                                   |
| `layout`     | Monocle/Columns/Rows layouts selecting grid cells per area            |
| `position`   | Converts selected cells into pixel rectangles                         |
| `tiling`     | Tiling engine: assignment, shift logic, tab stacking, position check  |
| `config`     | Configuration schema                                                  |
| `ipc`        | Named pipe server                                                     |
| `tray`       | System tray icon                                                      |
| `app`        | Main loop and command dispatch                                        |

## Building

```sh
# Windows 10 (default)
cargo build

# Windows 11
cargo build --no-default-features --features windows11
```

## Limitations

### Visual Blink on Desktop Switch

The application may blink when switching desktops with arguments.
Therefore, it is recommended to use built-in Windows shortcuts for desktop
switching and remap them in AHK if custom shortcuts are needed. FancyTWM still
provides this functionality.

### Windows 10 Virtual Desktop Offset

In Windows 10, there is a need to add one extra Virtual Desktop because the
library used under the hood does not return the last Virtual Desktop.

### Monitor arrangement

Only monitors arranged horizontally (left/right) are supported for cross-monitor
window movement. Vertically stacked monitors are not supported.
