# FancyTWM

FancyTWM is a tiling window manager built on top of PowerToys and FancyZones.
FancyZones provides the engine and grid-based layouts (e.g., 4×4 zones),
while FancyTWM controls window placement by setting tags that FancyZones reads.
It also handles all keybindings for moving windows and switching virtual desktops.

## Motivation

FancyZones is an excellent tool, but it lacks virtual desktop support and relies on a grid-based layout system.
FancyTWM bridges this gap by combining the best of both worlds.

## Installation and Configuration

### PowerToys

A pre-requirement is the installation of [PowerToys](https://learn.microsoft.com/windows/powertoys/).
Please follow their documentation to install this application and learn about features like `Fancy Zones` and `Keyboard Manager`.

### Configuration

Copy the example configuration file [example.toml](example/example.toml) to `%APPDATA%\FancyTWM\config.toml` and customize it to suit your preferences.

### Binaries

Pre-built binaries are available through GitHub Actions.
Download the latest ZIP archive, extract it, and place `fancytwm.exe` and `fancyctl.exe` in a directory of your choice.

Execute both applications before proceeding, as Windows will display a warning about the unknown publisher.
This warning appears because the binaries are not signed and the application has low usage.
To use them properly, confirm the exception when prompted.

### Key Bindings

The recommended approach is to configure keys via [Keyboard Manager](https://learn.microsoft.com/en-us/windows/powertoys/keyboard-manager#remap-a-shortcut-to-start-an-app), which uses `fancyctl.exe` via the command line to control certain actions.
Those are parameters which can be configured:

| Option     | Value                              |
| ---------- | ---------------------------------- |
| App        | Path to `fancyctl.exe` application |
| Args       | Command and all arguments          |
| Start in   | Default value                      |
| Elevation  | Normal                             |
| If running | Do nothing                         |
| Visibility | Hidden                             |

#### Example

| Option     | Value                       |
| ---------- | --------------------------- |
| App        | `C:\Apps\fancyctl.exe`      |
| Args       | `MoveToVirtualDesktop -i 0` |
| Start in   | Default value               |
| Elevation  | Normal                      |
| If running | Do nothing                  |
| Visibility | Hidden                      |

## FancyTWM client

Application `fancyctl.exe` is a client that allows communication with `fancytwm.exe` via a named pipe at the path `\\.\pipe\fancytwm-pipe`.
Communication based on JSON payload with following format:

```json
{
    "command": "string",
    "args": ["string"]
}
```

The following commands are available:

| Command                    | Args                                                 | Description                                                |
| -------------------------- | ---------------------------------------------------- | ---------------------------------------------------------- |
| `MoveToNextVirtualDesktop` | -                                                    | Move active window to next virtual desktop                 |
| `MoveToPrevVirtualDesktop` | -                                                    | Move active window to previous virtual desktop             |
| `MoveToVirtualDesktop`     | list with single element, index of desktop as string | Move active window to virtual desktop represented by index |

**Important**: Virtual desktops are enumerated from 0.

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
- [x] FancyTWM client
- [ ] Tagging logic
- [ ] Move within monitor logic
- [ ] Move across monitors logic
- [ ] New window tagging (if needed)

## Limitations

### Visual Blink on Desktop Switch

The application may blink when switching desktops with arguments.
Therefore, it is recommended to use built-in Windows shortcuts for desktop switching and remap them in AHK if custom shortcuts are needed.
FancyTWM still provides this functionality.

### Windows 10 Virtual Desktop Offset

In Windows 10, there is a need to add one extra Virtual Desktop because the library used under the hood does not return the last Virtual Desktop.
