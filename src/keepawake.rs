pub struct KeepAwake {
    #[cfg(unix)]
    inhibitor: Option<std::process::Child>,
}

impl KeepAwake {
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Power::{
                ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
            };
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
            }
            Self {}
        }
        #[cfg(unix)]
        {
            use std::process::{Command, Stdio};
            let mut cmd = if cfg!(target_os = "macos") {
                let mut c = Command::new("caffeinate");
                c.arg("-i").arg("-w").arg(std::process::id().to_string());
                c
            } else {
                let mut c = Command::new("systemd-inhibit");
                c.args([
                    "--what=idle:sleep",
                    "--who=Onyx",
                    "--why=Uploading to Steam",
                    "sleep",
                    "86400",
                ]);
                c
            };
            let inhibitor = cmd
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok();
            Self { inhibitor }
        }
        #[cfg(not(any(windows, unix)))]
        Self {}
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Power::{ES_CONTINUOUS, SetThreadExecutionState};
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS);
            }
        }
        #[cfg(unix)]
        if let Some(mut child) = self.inhibitor.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
