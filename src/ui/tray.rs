//! Notification-area icon shown while Onyx is hidden by "close to tray".
//!
//! The icon lives on its own thread with a hidden window, so it does not
//! depend on winit's window procedure. Clicks come back over a channel and wake
//! the UI with `request_repaint`; eframe keeps running `App::logic` while the
//! main window is hidden, which is what lets schedules keep firing.

use std::sync::mpsc::{Receiver, channel};

use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayEvent {
    Show,
    Quit,
}

pub struct Tray {
    rx: Receiver<TrayEvent>,
    #[cfg(windows)]
    hwnd: std::sync::Arc<std::sync::atomic::AtomicIsize>,
}

impl Tray {
    /// Add the icon. Returns `None` where tray icons are not supported.
    pub fn spawn(ctx: &egui::Context) -> Option<Self> {
        #[cfg(windows)]
        {
            let (tx, rx) = channel();
            let hwnd = std::sync::Arc::new(std::sync::atomic::AtomicIsize::new(0));
            let slot = hwnd.clone();
            let ctx = ctx.clone();
            std::thread::Builder::new()
                .name("onyx-tray".into())
                .spawn(move || win::run(tx, ctx, slot))
                .ok()?;
            Some(Self { rx, hwnd })
        }
        #[cfg(not(windows))]
        {
            let _ = (ctx, channel::<TrayEvent>);
            None
        }
    }

    pub fn poll(&self) -> Vec<TrayEvent> {
        self.rx.try_iter().collect()
    }
}

#[cfg(windows)]
impl Drop for Tray {
    fn drop(&mut self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
        let hwnd = self.hwnd.load(std::sync::atomic::Ordering::SeqCst);
        if hwnd != 0 {
            // SAFETY: posting WM_CLOSE to our own message window; it removes the icon
            // and ends the tray thread's message loop.
            unsafe {
                PostMessageW(hwnd as _, WM_CLOSE, 0, 0);
            }
        }
    }
}

#[cfg(windows)]
mod win {
    use std::cell::RefCell;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::mpsc::Sender;

    use eframe::egui;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Shell::{
        NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    use super::TrayEvent;

    const WM_TRAY: u32 = WM_APP + 1;
    const CMD_SHOW: usize = 1;
    const CMD_QUIT: usize = 2;

    struct State {
        tx: Sender<TrayEvent>,
        ctx: egui::Context,
        icon: HICON,
        taskbar_created: u32,
    }

    thread_local! {
        static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn send(ev: TrayEvent) {
        STATE.with(|st| {
            if let Some(st) = st.borrow().as_ref() {
                let _ = st.tx.send(ev);
                st.ctx.request_repaint();
            }
        });
    }

    fn icon_data(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
        // SAFETY: NOTIFYICONDATAW is plain data; all-zero is a valid starting value.
        let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = icon;
        for (dst, src) in nid.szTip.iter_mut().zip(wide("Onyx").iter()) {
            *dst = *src;
        }
        nid
    }

    fn add_icon(hwnd: HWND) {
        let icon = STATE.with(|st| st.borrow().as_ref().map(|s| s.icon));
        if let Some(icon) = icon {
            let nid = icon_data(hwnd, icon);
            // SAFETY: `nid` is fully initialised and outlives the call.
            unsafe {
                Shell_NotifyIconW(NIM_ADD, &nid);
            }
        }
    }

    fn show_menu(hwnd: HWND) {
        // SAFETY: standard popup-menu sequence on our own window. SetForegroundWindow
        // and the trailing WM_NULL are the documented way to make the menu close
        // when the user clicks elsewhere.
        unsafe {
            let menu = CreatePopupMenu();
            if menu.is_null() {
                return;
            }
            let show = wide("Show Onyx");
            let quit = wide("Quit Onyx");
            AppendMenuW(menu, MF_STRING, CMD_SHOW, show.as_ptr());
            AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
            AppendMenuW(menu, MF_STRING, CMD_QUIT, quit.as_ptr());
            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            SetForegroundWindow(hwnd);
            let cmd = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
                pt.x,
                pt.y,
                0,
                hwnd,
                std::ptr::null(),
            );
            PostMessageW(hwnd, WM_NULL, 0, 0);
            DestroyMenu(menu);
            match cmd as usize {
                CMD_SHOW => send(TrayEvent::Show),
                CMD_QUIT => send(TrayEvent::Quit),
                _ => {}
            }
        }
    }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let taskbar_created = STATE.with(|st| st.borrow().as_ref().map_or(0, |s| s.taskbar_created));
        match msg {
            WM_TRAY => {
                match (lparam as u32) & 0xFFFF {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => send(TrayEvent::Show),
                    WM_RBUTTONUP | WM_CONTEXTMENU => show_menu(hwnd),
                    _ => {}
                }
                0
            }
            WM_CLOSE => {
                // SAFETY: destroying our own window from its own thread.
                unsafe { DestroyWindow(hwnd) };
                0
            }
            WM_DESTROY => {
                let icon = STATE.with(|st| st.borrow().as_ref().map(|s| s.icon));
                if let Some(icon) = icon {
                    let nid = icon_data(hwnd, icon);
                    // SAFETY: removes the icon we added; `nid` identifies it by hWnd + uID.
                    unsafe {
                        Shell_NotifyIconW(NIM_DELETE, &nid);
                        DestroyIcon(icon);
                    }
                }
                // SAFETY: ends this thread's message loop.
                unsafe { PostQuitMessage(0) };
                0
            }
            // Explorer restarted: the notification area forgot our icon.
            m if m != 0 && m == taskbar_created => {
                add_icon(hwnd);
                0
            }
            // SAFETY: default handling for everything else.
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    fn make_icon() -> HICON {
        let data = crate::ui::window::app_icon();
        let (w, h) = (data.width as i32, data.height as i32);
        // CreateIcon wants BGRA colour bits plus a 1-bpp AND mask (all zero: use alpha).
        let mut bgra = data.rgba.clone();
        for px in bgra.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        let mask = vec![0u8; ((w + 15) / 16 * 2 * h) as usize];
        // SAFETY: buffers match the declared size and bit depth.
        unsafe {
            CreateIcon(
                GetModuleHandleW(std::ptr::null()),
                w,
                h,
                1,
                32,
                mask.as_ptr(),
                bgra.as_ptr(),
            )
        }
    }

    pub(super) fn run(tx: Sender<TrayEvent>, ctx: egui::Context, slot: Arc<AtomicIsize>) {
        let class = wide("OnyxTrayWindow");
        // SAFETY: registers a class and creates a message-only window owned by this
        // thread, then pumps its messages until WM_DESTROY posts WM_QUIT.
        unsafe {
            let hinst = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: hinst,
                lpszClassName: class.as_ptr(),
                // SAFETY: remaining fields are optional and valid when zeroed.
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            let created = wide("TaskbarCreated");
            STATE.with(|st| {
                *st.borrow_mut() = Some(State {
                    tx,
                    ctx,
                    icon: make_icon(),
                    taskbar_created: RegisterWindowMessageW(created.as_ptr()),
                })
            });
            let title = wide("Onyx tray");
            // A never-shown top-level window rather than HWND_MESSAGE: message-only
            // windows miss the TaskbarCreated broadcast and popup menus misbehave on them.
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinst,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                return;
            }
            slot.store(hwnd as isize, Ordering::SeqCst);
            add_icon(hwnd);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            STATE.with(|st| *st.borrow_mut() = None);
        }
    }
}
