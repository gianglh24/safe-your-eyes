//! Cross-platform user input idle duration provider (`sye_core::IdleProvider`).

#[cfg(target_os = "linux")]
use std::sync::Mutex;
use std::time::Duration;

use sye_core::{IdleProvider, PlatformError};

/// OS user-idle provider supporting Linux (X11/XWayland + Wayland D-Bus), macOS, and Windows.
pub struct SystemIdleProvider {
    #[cfg(target_os = "linux")]
    x11: Mutex<Option<linux_impl::X11IdleConn>>,
}

impl std::fmt::Debug for SystemIdleProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemIdleProvider").finish_non_exhaustive()
    }
}

impl SystemIdleProvider {
    /// Creates a new `SystemIdleProvider`.
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            x11: Mutex::new(None),
        }
    }
}

impl Default for SystemIdleProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IdleProvider for SystemIdleProvider {
    fn idle_duration(&self) -> Result<Duration, PlatformError> {
        #[cfg(target_os = "linux")]
        {
            linux_impl::query_idle_duration(&self.x11)
        }
        #[cfg(target_os = "macos")]
        {
            macos_impl::query_idle_duration()
        }
        #[cfg(target_os = "windows")]
        {
            windows_impl::query_idle_duration()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(PlatformError::Unsupported)
        }
    }
}

/// Parses `HIDIdleTime` (nanoseconds) from `ioreg -c IOHIDSystem -d 4` output on macOS.
///
/// Compiled unconditionally so cross-platform unit tests run on every OS.
pub fn parse_macos_hid_idle_time(output: &str) -> Option<Duration> {
    for line in output.lines() {
        if !line.contains("\"HIDIdleTime\"") {
            continue;
        }
        let (_, rhs) = line.split_once('=')?;
        let trimmed = rhs.trim();
        let nanos = trimmed.parse::<u64>().ok()?;
        return Some(Duration::from_nanos(nanos));
    }
    None
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use std::sync::Mutex;
    use std::time::Duration;

    use sye_core::PlatformError;
    use x11rb::connection::Connection as _;
    use x11rb::protocol::screensaver;
    use x11rb::protocol::xproto::Window;
    use x11rb::rust_connection::RustConnection;

    pub(super) struct X11IdleConn {
        conn: RustConnection,
        root: Window,
    }

    impl X11IdleConn {
        fn connect() -> Option<Self> {
            let (conn, screen_num) = RustConnection::connect(None).ok()?;
            let root = conn.setup().roots.get(screen_num)?.root;
            Some(Self { conn, root })
        }

        fn query_ms(&self) -> Option<u32> {
            let reply = screensaver::query_info(&self.conn, self.root)
                .ok()?
                .reply()
                .ok()?;
            Some(reply.ms_since_user_input)
        }
    }

    pub(super) fn query_idle_duration(
        cache: &Mutex<Option<X11IdleConn>>,
    ) -> Result<Duration, PlatformError> {
        if let Some(dur) = query_x11_idle(cache) {
            return Ok(dur);
        }
        if let Some(dur) = query_wayland_dbus_idle() {
            return Ok(dur);
        }
        Err(PlatformError::Unavailable(
            "no X11 display or session D-Bus idle monitor available".to_owned(),
        ))
    }

    fn query_x11_idle(cache: &Mutex<Option<X11IdleConn>>) -> Option<Duration> {
        let mut guard = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(existing) = guard.as_ref()
            && let Some(ms) = existing.query_ms()
        {
            return Some(Duration::from_millis(u64::from(ms)));
        }

        // Reconnect once if no cached connection or the previous connection errored.
        *guard = X11IdleConn::connect();
        let ms = guard.as_ref()?.query_ms()?;
        Some(Duration::from_millis(u64::from(ms)))
    }

    fn query_wayland_dbus_idle() -> Option<Duration> {
        let conn = zbus::blocking::Connection::session().ok()?;

        // GNOME Mutter IdleMonitor returns idle time in milliseconds as u64.
        if let Ok(msg) = conn.call_method(
            Some("org.gnome.Mutter.IdleMonitor"),
            "/org/gnome/Mutter/IdleMonitor/Core",
            Some("org.gnome.Mutter.IdleMonitor"),
            "GetIdletime",
            &(),
        ) && let Ok(ms) = msg.body().deserialize::<u64>()
        {
            return Some(Duration::from_millis(ms));
        }

        // Freedesktop ScreenSaver fallback returns idle time in milliseconds as u32.
        if let Ok(msg) = conn.call_method(
            Some("org.freedesktop.ScreenSaver"),
            "/org/freedesktop/ScreenSaver",
            Some("org.freedesktop.ScreenSaver"),
            "GetSessionIdleTime",
            &(),
        ) && let Ok(ms) = msg.body().deserialize::<u32>()
        {
            return Some(Duration::from_millis(u64::from(ms)));
        }

        None
    }
}

#[cfg(target_os = "macos")]
mod macos_impl {
    use std::process::Command;
    use std::time::Duration;

    use sye_core::PlatformError;

    use super::parse_macos_hid_idle_time;

    pub(super) fn query_idle_duration() -> Result<Duration, PlatformError> {
        let output = Command::new("ioreg")
            .args(["-c", "IOHIDSystem", "-d", "4"])
            .output()
            .map_err(|err| PlatformError::Unavailable(err.to_string()))?;

        if !output.status.success() {
            return Err(PlatformError::Unavailable(
                "ioreg exited with non-zero status".to_owned(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_macos_hid_idle_time(&stdout).ok_or_else(|| {
            PlatformError::Unavailable("HIDIdleTime not found in ioreg output".to_owned())
        })
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::mem::size_of;
    use std::time::Duration;

    use sye_core::PlatformError;
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    #[expect(unsafe_code, reason = "Win32 FFI")]
    pub(super) fn query_idle_duration() -> Result<Duration, PlatformError> {
        let mut lii = LASTINPUTINFO {
            cbSize: size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        // SAFETY: `lii` is properly initialized with its exact byte size and valid for writes.
        let ok = unsafe { GetLastInputInfo(&mut lii) };
        if ok == 0 {
            return Err(PlatformError::Unavailable(
                "GetLastInputInfo failed".to_owned(),
            ));
        }
        // SAFETY: `GetTickCount` has no preconditions and returns monotonic 32-bit ms since boot.
        let now_ms = unsafe { GetTickCount() };
        let elapsed_ms = now_ms.wrapping_sub(lii.dwTime);
        Ok(Duration::from_millis(u64::from(elapsed_ms)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_macos_hid_idle_time_extracts_nanoseconds() {
        let sample = r#"
          | |   "HIDIdleTime" = 4250000000
          | |   "HIDParameters" = {}
        "#;
        assert_eq!(
            parse_macos_hid_idle_time(sample),
            Some(Duration::from_nanos(4_250_000_000))
        );

        assert_eq!(
            parse_macos_hid_idle_time(r#""HIDIdleTime" = 0"#),
            Some(Duration::ZERO)
        );
        assert_eq!(parse_macos_hid_idle_time("no idle key here"), None);
        assert_eq!(parse_macos_hid_idle_time(r#""HIDIdleTime" = abc"#), None);
    }

    #[test]
    fn system_idle_provider_degrades_gracefully_in_headless_env() {
        let provider = SystemIdleProvider::new();
        let res = provider.idle_duration();
        assert!(matches!(res, Ok(_) | Err(PlatformError::Unavailable(_))));
    }
}
