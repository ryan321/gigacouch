# Wired USB controllers on macOS

macOS Game Controller / HID already covers DualShock 4, DualSense, Xbox Bluetooth, Switch, and most HID USB pads. Godot sees those without a helper. Press A / Cross to join.

Some cheap “PS4/PC” and Xbox 360-style wired pads talk **Xbox 360 XID** (`ff:5d:01`). Wired Xbox One / Series and many PowerA/PDP/8BitDo Xbox pads talk **GIP** (`ff:47:d0`). Darwin binds neither, so Little World would not see them. The host helper `tools/macos/xpad_reader.m` claims those devices, decodes input, and publishes `COUCH_XPAD_NATIVE_STATE`. `native_xpad.gd` turns that file into standard gamepad events.

This is the same host-file pattern as the native Wii reader. It is not a kernel driver and it is not used for HID pads.

**Chrome does not see these pads on a Mac.** The helper only feeds the Godot player. Chrome's Mac driver for Xbox-style pads covers a fixed list of models, mostly Microsoft's own, so third-party XID and GIP pads such as the Nacon PC Compact (`146b:0603`) are invisible to web games. [v1.md](v1.md#wired-xbox-style-pads-on-macos) has the details and the pads that do work.

## Run

```sh
python3 scripts/play.py
```

If a vendor-class USB pad is plugged in, play.py starts the helper. Keep the **wired USB reader** window open. `python3 scripts/play_xpad_native.py --check` only confirms reports.

Quit Chrome/Brave if they grabbed the USB device. F3 shows whether the helper is live.

## What is claimed

The helper walks every `IOUSBHostDevice`, reads its configuration descriptor **before** `SetConfiguration`, and claims a pad when `xpad_lookup` in `tools/macos/xpad_devices.h` returns `XPAD_PROTO_XID360` or `XPAD_PROTO_GIP`. HID interfaces (class 3), hubs, NICs, and disks are skipped. Official Xbox pads that macOS already exposes as HID/Game Controller are left alone.

Unknown VID/PID still matches XID or GIP by interface class. Named rows override the generic name and can set quirks. GIP pads get a power-on / LED / auth init sequence (plus One S and some PowerA extras).

## Unknown pad (for a person or an agent)

Quit Little World, Chrome, and Brave so nothing holds USB. Plug the pad in over USB, then:

```sh
python3 scripts/play_xpad_native.py --dump
```

Read the JSON:

| `match` | What to do |
| --- | --- |
| `xid360` or `gip` | Already claimed. Relaunch `python3 scripts/play.py` and press A. Add `catalog_row` to `tools/macos/xpad_devices.h` only to name it or set quirks. |
| `hid-leave-to-godot` | macOS/Godot should see it. No helper row. If buttons are wrong, that is an SDL mapping issue, not USB. |
| `none` | New protocol. Keep the dump. Add `XPAD_PROTO_…` and a decoder in `xpad_reports.h`, then a named row. Do not put USB code in the Godot game. |

Paste the dump into an agent with: add this pad to `tools/macos/xpad_devices.h` using `catalog_row`, or implement a decoder if `match` is `none`. Rebuild is automatic on the next `play.py`.

GIP pads get power-on, One S init, LED, auth, and a short rumble init so PowerA/PDP clones are more likely to start sending reports. HORI-style analog-stick ack is sent for HORI/Titanfall VID/PIDs.

Xbox One/Series **wired** GIP is implemented; it has not been physically pressed on this Mac. Bluetooth Xbox pads already work through Godot and do not use this reader. Touchpad, headset jack, and rumble-as-a-feature are out of scope.
