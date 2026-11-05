//! Loaded into GUI apps: while a window manager says it's resizing windows, windows that get resized
//! are put into a live resize, the same one AppKit runs for a drag of the window's edge.
//!
//! An app only learns that a resize is going on from `-[NSWindow _startLiveResize]`, which AppKit
//! calls from its own mouse tracking. A window resized from outside, over Accessibility, just sees
//! one unrelated frame change after another, and apps redraw accordingly.
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::{CStr, c_int, c_void};
use core::mem::transmute;
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

pub(crate) use eris_inject_payload::{log, msg, objc, rt};

use eris_inject_payload::sys::{SpinLock, notify_get_state, notify_register_check, observe_darwin, on_main};
use objc::{Id, Imp, Original, Sel, class, define_class, release, retain, swizzle};

eris_inject_payload::payload!(name: "liveresize", log: "liveresize", start: start);

/// Posted, with a state of 1 or 0, as a window manager starts and stops resizing windows.
const NOTIFICATION: &CStr = c"io.github.eriskii.liveresize.active";

/// Defined once we're running, so that loading twice into an app doesn't hook twice.
const LOADED_CLASS: &CStr = c"ErisLiveResizeLoaded";

const NS_WINDOW_STYLE_MASK_TITLED: usize = 1;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static TOKEN: AtomicI32 = AtomicI32::new(-1);

struct Window(Id);

// Only ever used on the main thread.
unsafe impl Send for Window {}

/// Windows we started a live resize for, retained.
static STARTED: SpinLock<Vec<Window>> = SpinLock::new(Vec::new());

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// `-[NSWindow setFrame:display:]` is what an Accessibility resize ends up calling.
static SET_FRAME: Original = Original::new();

extern "C" fn set_frame(this: Id, cmd: Sel, frame: CGRect, display: bool) {
    let original: extern "C" fn(Id, Sel, CGRect, bool) = unsafe { transmute(SET_FRAME.get()) };
    if ACTIVE.load(Ordering::Acquire) {
        begin(this);
    }
    original(this, cmd, frame, display)
}

fn begin(window: Id) {
    if msg!(usize; window, c"styleMask") & NS_WINDOW_STYLE_MASK_TITLED == 0 {
        return;
    }
    let view = msg!(Id; window, c"contentView");
    // Already in one: AppKit's own, for the window whose edge is being dragged, or ours.
    if view.is_null() || msg!(bool; view, c"inLiveResize") {
        return;
    }
    STARTED.with(|started| started.push(Window(retain(window))));
    msg!((); window, c"_startLiveResize");
}

extern "C" fn finish(_: *mut c_void) {
    if ACTIVE.load(Ordering::Acquire) {
        return;
    }
    for Window(window) in STARTED.with(core::mem::take) {
        msg!((); window, c"_endLiveResize");
        release(window);
    }
}

fn adopt_state() {
    let mut state = 0;
    let token = TOKEN.load(Ordering::Relaxed);
    if token < 0 || unsafe { notify_get_state(token, &mut state) } != 0 {
        return;
    }
    ACTIVE.store(state == 1, Ordering::Release);
    if state != 1 {
        on_main(finish);
    }
}

extern "C" fn changed(_: *mut c_void, _: *const c_void, _: *const c_void, _: *const c_void, _: *const c_void) {
    adopt_state();
}

fn start(injected: bool) {
    let wait_for_appkit_ms = if injected { 10_000 } else { 0 };
    let pid = unsafe { rt::getpid() };
    if !class(LOADED_CLASS).is_null() {
        log!("already running in pid {pid}");
        return;
    }
    // The daemon loads us as soon as an app starts, which can be before AppKit is up.
    let mut waited = 0;
    while class(c"NSWindow").is_null() {
        if waited >= wait_for_appkit_ms {
            log!("no AppKit in pid {pid}; nothing to do");
            return;
        }
        unsafe { rt::usleep(50_000) };
        waited += 50;
    }
    define_class(LOADED_CLASS, c"NSObject", &[]);

    let mut token: c_int = -1;
    if unsafe { notify_register_check(NOTIFICATION.as_ptr(), &mut token) } == 0 {
        TOKEN.store(token, Ordering::Relaxed);
    }
    observe_darwin(NOTIFICATION, changed);

    if !swizzle(c"NSWindow", c"setFrame:display:", set_frame as Imp, &SET_FRAME) {
        log!("missing NSWindow.setFrame:display:; nothing to do");
        return;
    }
    adopt_state();
    log!("running in pid {pid}");
}
