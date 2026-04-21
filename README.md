# FancyTWM

FancyTWM is a tiling window manager built on top of PowerToys and FancyZones.
FancyZones provides the engine and grid-based layouts (e.g., 4×4 zones),
while FancyTWM controls window placement by setting tags that FancyZones reads.
It also handles all keybindings for moving windows and switching virtual desktops.

## Motivation

FancyZones is an excellent tool, but it lacks virtual desktop support and relies on a grid-based layout system.
FancyTWM bridges this gap by combining the best of both worlds.

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
- [ ] Virtual desktop management
  - Move to specific virtual desktop
- [ ] FancyTWM client (fancyctl)
- [ ] FancyTWM server
- [ ] AHK keybindings
- [ ] Tagging logic
- [ ] Move within monitor logic
- [ ] Move across monitors logic
- [ ] New window tagging (if needed)
