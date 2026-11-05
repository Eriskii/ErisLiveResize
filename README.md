# ErisLiveResize

Real live resizes on macOS for windows resized by a window manager.

An app only learns that a resize is going on from `-[NSWindow _startLiveResize]`, which AppKit calls
from its own mouse tracking when you drag a window's edge. A window resized from outside, over
Accessibility, just sees one unrelated frame change after another, and apps redraw accordingly: kitty
blanks its content and shows no size hint, for one.

A small payload, loaded into every app through
[ErisMacInjectLib](https://github.com/Eriskii/ErisMacInjectLib), does what the edge drag does: while a
window manager says it's resizing windows, any window that gets resized is put into a live resize,
and taken out of it afterwards. The mouse isn't involved, so this also covers the windows next to the
one being dragged, which a real edge drag can't.

## The signal

A Darwin notification, with its state saying whether a resize is in progress:

```sh
notifyutil -s io.github.eriskii.liveresize.active 1
notifyutil -p io.github.eriskii.liveresize.active
```

[Eriskii/yabai](https://github.com/Eriskii/yabai) sets it during a drag. Without a window manager
setting it, the payload does nothing.

## Requirements

- Apple Silicon, macOS 26
- SIP partially disabled (debugging restrictions off) and the `-arm64e_preview_abi` boot-arg
- Nightly Rust, pinned in `rust-toolchain.toml`

## Install

```sh
make
sudo target/dist/liveresize install
```

The daemon loads the payload into running apps and into each app as it starts. An app that already
has a copy keeps it until it's relaunched. `sudo liveresize uninstall` removes the daemon, and apps go
back to normal as they're relaunched.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
