use crate::{
    errors::*,
    input::keyboard::{Adapter as KbdAdapter, Keyboard},
    invoke::chk,
    types::*,
    window::{Theme, WindowClass, DPI},
};

use ::parking_lot::RwLock;
use ::std::{
    cell::{Cell, UnsafeCell},
    marker::PhantomData,
    ops::DerefMut,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use ::tracing::debug;
use ::widestring::U16CString;
use ::win_geom::d2::{Point2D, Rect2D, Size2D};
use ::windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::{
            Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE},
            Gdi::UpdateWindow,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::AdjustWindowRectExForDpi,
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW,
                SetWindowLongPtrW, SetWindowPos, ShowWindow, CREATESTRUCTW, CW_USEDEFAULT,
                GWLP_USERDATA, GWLP_WNDPROC, SWP_NOMOVE, SW_SHOWNORMAL, WINDOW_EX_STYLE, WM_CLOSE,
                WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WS_OVERLAPPEDWINDOW,
            },
        },
    },
};

pub(super) struct WindowInner {
    /// Force !Send & !Sync, as our window can only be used by the thread on
    /// which it was created.
    phantom: PhantomData<UnsafeCell<()>>,
    /// A reference-counted handle to the Win32 window class registered for
    /// windows of this type. When the last `Window` instance is released, the
    /// corresponding Win32 window class will be de-registered.
    window_class: Arc<WindowClass>,
    /// A handle to our corresponding Win32 window. If zero, the window has been
    /// destroyed on the Win32 size.
    hwnd: Cell<HWND>,
    /// Fixed size for our window's client area.
    size: Size2D<i32>,
    /// The Window's title, as it appears in the Windows title bar.
    title: String,
    /// The system theme in use by the window - "light" or "dark". This does not
    /// auto-update to track the true system value yet.
    theme: Cell<Theme>,
    /// Stores an outstanding close request from the Win32 side. This must
    /// either be actioned by dropping the top level window, or the close
    /// request can be cleared if it is to be ignored.
    close_request: AtomicBool,
    /// Stores an outstanding paint request from the Win32 side.
    redraw_request: AtomicBool,
    /// Keyboard and text input state.
    keyboard: RwLock<Keyboard>,
}

impl WindowInner {
    /// Construct and display a new window.
    pub(super) fn new(
        size: Size2D<i32>,
        title: &str,
        icon_id: Option<ResourceId>,
        theme: Theme,
    ) -> Result<Rc<Self>> {
        debug!(wnd_title = %title, "Creating window inner");

        let this = Rc::new(Self {
            phantom: Default::default(),
            title: title.to_string(),
            window_class: WindowClass::get_or_create("MainWindow", icon_id, Self::wnd_proc_setup)?,
            hwnd: Default::default(),
            size,
            theme: Cell::new(theme),
            close_request: AtomicBool::new(false),
            redraw_request: AtomicBool::new(true), // Request immediate draw
            keyboard: RwLock::new(Keyboard::new()),
        });

        let hwnd = {
            let module = chk!(res; GetModuleHandleW(None))?;
            let title = U16CString::from_str(title).expect("Window name contained null byte");

            chk!(ptr; CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    PCWSTR::from_raw(this.window_class.class_name().as_ptr()),
                    PCWSTR::from_raw(title.as_ptr()),
                    WS_OVERLAPPEDWINDOW,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    // 0 pixel width/height: show window as hidden first so we
                    // can detect the monitor's DPI:
                    0,
                    0,
                    None,
                    None,
                    module,
                    Some(Rc::into_raw(this.clone()) as *const _)
                )
            )?
        };
        this.hwnd.set(hwnd);

        // `SetWindowPos` function takes its size in pixels, so we
        // obtain the window's DPI and use it to scale the window size_
        let dpi = DPI::detect(hwnd);
        let mut rect = dpi
            .scale_rect(Rect2D::from_size_and_origin(size, Point2D::zero()))
            .into();
        chk!(bool; AdjustWindowRectExForDpi(
            &mut rect,
            WS_OVERLAPPEDWINDOW,
            false,
            WINDOW_EX_STYLE::default(),
            dpi.into()
        ))?;

        let pixel_width = rect.right - rect.left;
        let pixel_height = rect.bottom - rect.top;
        ::tracing::warn!("adjusted window size: {pixel_width} x {pixel_height}");

        chk!(bool; SetWindowPos(
            hwnd,
            HWND::default(),
            0,
            0,
            pixel_width,
            pixel_height,
            SWP_NOMOVE
        ))?;

        this.set_theme(theme);
        unsafe {
            ShowWindow(hwnd, SW_SHOWNORMAL);
            UpdateWindow(hwnd);
        }

        Ok(this)
    }

    /// The size of the client area of our Win32 window. The window chrome
    /// is in addition to this siz3.
    pub(super) const fn size(&self) -> Size2D<i32> {
        self.size
    }

    pub(super) fn title(&self) -> &str {
        &self.title
    }

    /// Get a handle to the Win32 window's handle. This is often required when
    /// interacting with other APIs.
    ///
    /// If `None`, then the window has already been destroyed on the Win32 side.
    pub(super) fn hwnd(&self) -> HWND {
        let val = self.hwnd.get();
        assert_ne!(val.0, 0, "Window handle was NULL");
        val
    }

    /// Sets the window's system theme. This currently only controls the color
    /// of the title bar.
    pub(super) fn current_theme(&self) -> Theme {
        self.theme.get()
    }

    /// Sets the window's title bar to match the given theme.
    pub(super) fn set_theme(&self, theme: Theme) {
        let val: i32 = match theme {
            Theme::DarkMode => 0x01,
            Theme::LightMode => 0x00,
        };

        self.theme.set(theme);

        chk!(res;
            DwmSetWindowAttribute(
                self.hwnd(),
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &val as *const i32 as _,
                ::std::mem::size_of::<i32>() as u32
            )
        )
        .unwrap();
    }

    /// Returns whether the window has requested to close, and immediately
    /// clears this request. Window is not actually closed until it is
    /// dropped, so the close request can be ignored if needed.
    pub(super) fn clear_close_request(&self) -> bool {
        self.close_request.swap(false, Ordering::SeqCst)
    }

    /// Returns whether the window has requested to redraw, and immediately
    /// clears this request. Window is not actually redrawn until it is painted
    /// by external higher level code, so the close request can be ignored if
    /// needed.
    pub(super) fn clear_redraw_request(&self) -> bool {
        self.redraw_request.swap(false, Ordering::SeqCst)
    }

    pub fn keyboard(&self) -> impl DerefMut<Target = Keyboard> + '_ {
        self.keyboard.write()
    }

    pub(super) fn destroy(&self) -> Result<()> {
        chk!(bool; DestroyWindow(self.hwnd()))?;
        Ok(())
    }

    /// Handles a Win32 message.
    ///
    /// ## Return Value
    ///
    /// Returns `true` if the message was handled and should not be forwarded to
    /// the default window procedure. Returns `false` if the message was not
    /// handled, or was only intercepted/tapped on the way though and should
    /// still be forwarded to the default procedure.
    fn handle_message(&self, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> bool {
        ::tracing::trace!(msg = %crate::debug::msgs::DebugMsg::new(umsg, wparam, lparam));

        if KbdAdapter::handles_msg(umsg, wparam, lparam) {
            if let Some(event) = KbdAdapter::adapt(umsg, wparam, lparam) {
                self.keyboard.write().process_evt(event);
            }
            return true;
        }

        match umsg {
            // TODO: WM_DPICHANGED (+send to d2d)
            WM_PAINT => {
                self.redraw_request.store(true, Ordering::SeqCst);
                false
            }
            WM_CLOSE => {
                self.close_request.store(true, Ordering::SeqCst);
                true
            }
            WM_NCDESTROY => {
                debug!(wnd_title = %self.title, "Destroying window inner");

                // Our window is being destroyed, so we must clean up our Rc'd
                // handle on the Win32 side.
                let self_ = chk!(last_err; SetWindowLongPtrW(self.hwnd(), GWLP_USERDATA, 0))
                    .unwrap() as *const Self;
                let _ = unsafe { Rc::from_raw(self_) };

                // Clear our window handle now that we're destroyed.
                self.hwnd.set(HWND(0));

                // forward to default procedure too
                false
            }
            _ => false,
        }
    }

    /// C-function Win32 window procedure performs one-time setup of the
    /// structures on the Win32 side to associate our Rust object with the Win32
    /// object.
    extern "system" fn wnd_proc_setup(
        hwnd: HWND,
        umsg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        // If we've received a create event, then we populate an `Rc`'ed
        // reference our rust window type in the user data section of the Win32
        // window.
        if umsg == WM_NCCREATE {
            let create_struct = lparam.0 as *const CREATESTRUCTW;
            // SAFETY: The `CREATESRUCTA` structure is guaranteed by the Win32
            // API to be valid if we've received an event of type `WM_NCCREATE`.
            let self_ = unsafe { (*create_struct).lpCreateParams } as *const Self;

            chk!(last_err; SetWindowLongPtrW(hwnd, GWLP_USERDATA, self_ as _)).unwrap();
            chk!(last_err; SetWindowLongPtrW(hwnd, GWLP_WNDPROC, (Self::wnd_proc_thunk as usize) as isize))
                .unwrap();
        }

        // We _always_ pass our message through to the default window procedure.
        unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) }
    }

    /// A minimal shim which forwards Win32 window proc messages to our own
    /// type for handling.
    extern "system" fn wnd_proc_thunk(
        hwnd: HWND,
        umsg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if let Ok(ptr) = chk!(nonzero_isize; GetWindowLongPtrW(hwnd, GWLP_USERDATA)) {
            let self_ = ptr.get() as *const Self;

            unsafe {
                // Add extra retain for the duration of following call
                Rc::increment_strong_count(self_);
                if Rc::from_raw(self_).handle_message(umsg, wparam, lparam) {
                    return LRESULT(0);
                }
            }
        }

        unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) }
    }
}
