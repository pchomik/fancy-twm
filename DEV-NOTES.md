# Dev Notes

## `#[allow(dead_code)]` markers

7 markers in 3 files. Each exists for a specific reason.

### Config fields deserialized but not read programmatically

- **`VirtualDesktop.name`** (`config.rs:52`) — User-facing label in TOML (e.g. `name = "Work"`). Code accesses virtual desktops by index, not by name.
- **`MonitorLayout.monitor`** (`config.rs:64`) — Optional monitor device name (`\\.\DISPLAY1`). Code resolves layouts by monitor position index. The field is parsed and available for future name-based monitor matching.

### Platform infrastructure not yet wired to commands

- **`MonitorInfo.name`** (`platform.rs:315`) — Populated during `EnumDisplayMonitors` but no consumer reads it. Counterpart to `MonitorLayout.monitor` — both halves of monitor-by-name matching exist, the join does not.
- **`Direction` enum** (`platform.rs:321`) — `Left` / `Right`. Used only by `get_adjacent_monitor()`.
- **`get_adjacent_monitor()`** (`platform.rs:430`) — Returns the neighboring monitor in a given direction. No command calls it. Intended for cross-monitor window movement (the app has cross-monitor tiling but not cross-monitor move/focus).
- **`VirtualDesktopTracker.desktop_count()`** (`platform.rs:485`) — Getter for a field already maintained on every poll. `current_index()` is called externally; `desktop_count()` is not.

### RAII guard

- **`TrayController.icon`** (`tray.rs:9`) — `TrayIcon` must be held in a struct field to stay alive. Dropping it removes the tray icon. The field is never read, only stored.
