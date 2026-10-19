//! Read only the primary button state while a native window is being moved.
//! OS title bars do not deliver their pointer releases to egui widgets.

#[cfg(target_os = "windows")]
pub(crate) fn primary_down() -> Option<bool> {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetAsyncKeyState(key: i32) -> i16;
    }
    // SAFETY: GetAsyncKeyState takes a virtual-key value and no pointers.
    Some(unsafe { GetAsyncKeyState(0x01) } < 0)
}

#[cfg(target_os = "macos")]
pub(crate) fn primary_down() -> Option<bool> {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceButtonState(state: i32, button: u32) -> bool;
    }
    // SAFETY: Query the combined session state (0), left mouse button (0).
    Some(unsafe { CGEventSourceButtonState(0, 0) })
}

#[cfg(target_os = "linux")]
pub(crate) fn primary_down() -> Option<bool> {
    use x11_dl::xlib;
    struct Pointer {
        api: xlib::Xlib,
        display: *mut xlib::Display,
    }
    impl Pointer {
        fn open() -> Option<Self> {
            let api = xlib::Xlib::open().ok()?;
            // SAFETY: A null name selects the current X display. The connection
            // is owned by this thread-local value and closed when it is dropped.
            let display = unsafe { (api.XOpenDisplay)(std::ptr::null()) };
            if display.is_null() {
                None
            } else {
                Some(Self { api, display })
            }
        }
        fn down(&self) -> Option<bool> {
            let (mut root, mut child) = (0, 0);
            let (mut root_x, mut root_y, mut x, mut y, mut mask) = (0, 0, 0, 0, 0);
            // SAFETY: The display is live on its owning thread; all output
            // pointers refer to initialized local variables of the Xlib types.
            let ok = unsafe {
                (self.api.XQueryPointer)(
                    self.display,
                    (self.api.XDefaultRootWindow)(self.display),
                    &mut root,
                    &mut child,
                    &mut root_x,
                    &mut root_y,
                    &mut x,
                    &mut y,
                    &mut mask,
                )
            };
            (ok != 0).then_some(mask & xlib::Button1Mask != 0)
        }
    }
    impl Drop for Pointer {
        fn drop(&mut self) {
            // SAFETY: This is the unique owner of the X display connection.
            unsafe {
                (self.api.XCloseDisplay)(self.display);
            }
        }
    }
    thread_local! { static POINTER: Option<Pointer> = Pointer::open(); }
    POINTER.with(|pointer| pointer.as_ref().and_then(Pointer::down))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub(crate) fn primary_down() -> Option<bool> {
    None
}
