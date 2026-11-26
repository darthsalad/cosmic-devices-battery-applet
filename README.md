# cosmic-applet-batteries

COSMIC panel applet that shows battery levels of connected devices
(mice, keyboards, headsets, controllers, phones) reported by UPower.

- Panel icon: battery level of the lowest device.
- Click: popup listing each device with a type icon, percentage and level icon.
- Updates on UPower D-Bus signals, with a 60 s fallback refresh.

Built with [libcosmic](https://github.com/pop-os/libcosmic) and
[zbus](https://github.com/dbus2/zbus).

## Screenshots

| Panel icon | Popup |
| --- | --- |
| ![Panel icon](docs/screenshots/panel.png) | ![Popup listing devices](docs/screenshots/popup.png) |

## Install

```
./install.sh
```

This builds a release binary and installs it to `~/.local/bin`, plus the
applet entry to `~/.local/share/applications`. Set `PREFIX` to change
the location.

Then add it to the panel: Settings > Desktop > Panel > Applets >
Device Batteries.

## Uninstall

```
./install.sh --uninstall
```

## Limitations

Only devices that expose battery data to UPower appear (Logitech HID++,
Bluetooth Battery Service, etc.). Many generic 2.4 GHz receivers do not
report battery level, so their devices are not listed.
