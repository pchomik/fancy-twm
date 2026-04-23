# FancyTWM

FancyTWM is a tiling window manager built on top of PowerToys and FancyZones.
FancyZones provides the engine and grid-based layouts (e.g., 4×4 zones),
while FancyTWM controls window placement by setting tags that FancyZones reads.
It also handles all keybindings for moving windows and switching virtual desktops.

## Motivation

FancyZones is an excellent tool, but it lacks virtual desktop support and relies on a grid-based layout system.
FancyTWM bridges this gap by combining the best of both worlds.

## Installation and Configuration

### AutoHotkey

FancyTWM requires **AutoHotkey v2** to be installed on your system.
Additionally, the FancyTWM AHK library must be placed in `%USERPROFILE%\Documents\AutoHotkey\Lib` to enable communication with the server.

### Configuration

Copy the example configuration file [example.toml](example/example.toml) to `%APPDATA%\FancyTWM\config.toml` and customize it to suit your preferences.

### Binaries

Pre-built binaries are available through GitHub Actions. Download the latest ZIP archive, extract it, and place `fancytwm.exe` in a directory of your choice.

### Key Bindings

Finally, configure key bindings for specific actions using an AHK script such as [example.ahk](example/example.ahk).
Copy this file to `%APPDATA%\FancyTWM\keybindings.ahk` and adjust the bindings to your liking.

## Features

### Layouts

Three tiling layouts are available:

| Layout      | Description                                                                                                      |
| ----------- | ---------------------------------------------------------------------------------------------------------------- |
| **Monocle** | A single window occupies the entire screen                                                                       |
| **Columns** | The master window takes the first zone; subsequent windows fill the remaining zones and stack in the last column |
| **Rows**    | The master window takes the first row; subsequent windows fill the remaining rows and stack in the last row      |

## Project Structure

| Workspace      | Purpose                                                                    |
| -------------- | -------------------------------------------------------------------------- |
| **fancy-twm**  | Core application for managing windows across monitors and virtual desktops |
| **fancy-ctl**  | Command-line tool for triggering actions                                   |
| **fancy-core** | Shared library containing code common to all FancyTWM components           |

## Roadmap

- [x] Configuration logic
- [x] Tray app
- [x] Layouts
- [x] Virtual desktop management
- [x] FancyTWM server
- [x] AHK library
- [x] AHK configuration
- [ ] Start keybindings.ahk if available
- [ ] Tagging logic
- [ ] Move within monitor logic
- [ ] Move across monitors logic
- [ ] New window tagging (if needed)
- [ ] FancyTWM client (fancyctl) ???

## Limitations

### Visual Blink on Desktop Switch

The application may blink when switching desktops with arguments.
Therefore, it is recommended to use built-in Windows shortcuts for desktop switching and remap them in AHK if custom shortcuts are needed.
FancyTWM still provides this functionality.

### Windows 10 Virtual Desktop Offset

In Windows 10, there is a need to add one extra Virtual Desktop because the library used under the hood does not return the last Virtual Desktop.
