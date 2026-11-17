//! Build progress on the Windows taskbar button (ITaskbarList3).
//!
//! windows-sys has no COM interface bindings, so the two methods used here are
//! called through the vtable by index. Everything runs on the UI thread, which
//! winit has already initialised for COM (OLE), as the STA object requires.

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Progress {
    None,
    Indeterminate,
    Normal(f32),
    Error,
}

pub struct Taskbar {
    #[cfg(windows)]
    hwnd: isize,
    #[cfg(windows)]
    list: *mut core::ffi::c_void,
    last: Option<Progress>,
}

impl Taskbar {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(windows)]
        {
            let (hwnd, list) = create(cc).unwrap_or((0, std::ptr::null_mut()));
            Self { hwnd, list, last: None }
        }
        #[cfg(not(windows))]
        {
            let _ = cc;
            Self { last: None }
        }
    }

    /// Update the taskbar button. Calls through to Windows only when the value changes.
    pub fn set(&mut self, progress: Progress) {
        let progress = match progress {
            // Quantise so a steadily climbing upload doesn't call COM every frame.
            Progress::Normal(f) => Progress::Normal((f.clamp(0.0, 1.0) * 200.0).round() / 200.0),
            other => other,
        };
        if self.last == Some(progress) {
            return;
        }
        self.last = Some(progress);
        #[cfg(windows)]
        self.apply(progress);
    }

    #[cfg(windows)]
    fn apply(&self, progress: Progress) {
        use windows_sys::Win32::UI::Shell::{TBPF_ERROR, TBPF_INDETERMINATE, TBPF_NOPROGRESS, TBPF_NORMAL};
        if self.list.is_null() || self.hwnd == 0 {
            return;
        }
        type SetValue = unsafe extern "system" fn(*mut core::ffi::c_void, isize, u64, u64) -> i32;
        type SetState = unsafe extern "system" fn(*mut core::ffi::c_void, isize, i32) -> i32;
        // SAFETY: `list` is a live ITaskbarList3 created in `create`. Slot 9 is
        // SetProgressValue and slot 10 SetProgressState (IUnknown 0-2, ITaskbarList 3-7,
        // ITaskbarList2 8). Both take the object pointer, an HWND and plain integers.
        unsafe {
            let vtbl = *(self.list as *const *const usize);
            let set_value: SetValue = std::mem::transmute(*vtbl.add(9));
            let set_state: SetState = std::mem::transmute(*vtbl.add(10));
            match progress {
                Progress::None => {
                    set_state(self.list, self.hwnd, TBPF_NOPROGRESS);
                }
                Progress::Indeterminate => {
                    set_state(self.list, self.hwnd, TBPF_INDETERMINATE);
                }
                Progress::Normal(f) => {
                    set_state(self.list, self.hwnd, TBPF_NORMAL);
                    set_value(self.list, self.hwnd, (f * 1000.0) as u64, 1000);
                }
                Progress::Error => {
                    set_state(self.list, self.hwnd, TBPF_ERROR);
                    set_value(self.list, self.hwnd, 1000, 1000);
                }
            }
        }
    }
}

#[cfg(windows)]
impl Drop for Taskbar {
    fn drop(&mut self) {
        if self.list.is_null() {
            return;
        }
        type Release = unsafe extern "system" fn(*mut core::ffi::c_void) -> u32;
        // SAFETY: slot 2 of every COM vtable is IUnknown::Release; we own one reference.
        unsafe {
            let vtbl = *(self.list as *const *const usize);
            let release: Release = std::mem::transmute(*vtbl.add(2));
            release(self.list);
        }
    }
}

#[cfg(windows)]
fn create(cc: &eframe::CreationContext<'_>) -> Option<(isize, *mut core::ffi::c_void)> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
    use windows_sys::core::GUID;

    const CLSID_TASKBAR_LIST: GUID = GUID::from_u128(0x56fdf344_fd6d_11d0_958a_006097c9a090);
    const IID_ITASKBAR_LIST3: GUID = GUID::from_u128(0xea1afb91_9e28_4b86_90e9_9e9f8a5eefaf);

    let RawWindowHandle::Win32(win) = cc.window_handle().ok()?.as_raw() else {
        return None;
    };
    let hwnd = win.hwnd.get();
    let mut list: *mut core::ffi::c_void = std::ptr::null_mut();
    type HrInit = unsafe extern "system" fn(*mut core::ffi::c_void) -> i32;
    // SAFETY: standard CoCreateInstance call with valid GUID pointers and an out
    // pointer; slot 3 of ITaskbarList is HrInit, which must be called before use.
    unsafe {
        let hr = CoCreateInstance(
            &CLSID_TASKBAR_LIST,
            std::ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_ITASKBAR_LIST3,
            &mut list,
        );
        if hr < 0 || list.is_null() {
            return None;
        }
        let vtbl = *(list as *const *const usize);
        let hr_init: HrInit = std::mem::transmute(*vtbl.add(3));
        if hr_init(list) < 0 {
            let release: unsafe extern "system" fn(*mut core::ffi::c_void) -> u32 =
                std::mem::transmute(*vtbl.add(2));
            release(list);
            return None;
        }
    }
    Some((hwnd, list))
}
