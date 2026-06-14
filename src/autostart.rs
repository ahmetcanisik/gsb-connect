//! "Start with Windows" support (per-user, no administrator rights).
//!
//! Auto-start is implemented with the per-user registry Run key:
//!   `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//! Enabling writes a string value named [`RUN_VALUE_NAME`] whose data is the
//! quoted full path to the current executable plus the `--autostart` flag;
//! disabling deletes that value. Because it lives under `HKEY_CURRENT_USER` it
//! needs no elevation and is cleanly removable (the user can also delete it
//! manually from Task Manager's Startup tab or `regedit`).
//!
//! The whole platform body is `#[cfg(windows)]`. On other platforms the public
//! functions are no-ops (auto-start reports as disabled) so the project still
//! builds and runs during development on Linux/macOS.
//!
//! The raw Win32 registry calls go through the already-present `windows-sys`
//! crate (the same lightweight dependency used for DPAPI in `secret_store`), so
//! this adds no new dependency; it only enables the `Win32_System_Registry`
//! feature.

use std::io;

/// Registry value name under the Run key. This is the user-visible name shown in
/// Task Manager's Startup tab; it intentionally matches the product name.
const RUN_VALUE_NAME: &str = "GSB Connect";

/// Command-line flag added to the registered command so a boot-time launch can
/// start minimized. `main` checks for the same flag.
pub const AUTOSTART_FLAG: &str = "--autostart";

/// Returns whether auto-start is currently enabled (the Run value exists).
///
/// On any error, or on non-Windows builds, returns `false`.
pub fn is_enabled() -> bool {
    imp::is_enabled()
}

/// Enables or disables auto-start by writing or deleting the Run value.
///
/// On non-Windows builds this is a successful no-op.
pub fn set_enabled(enabled: bool) -> io::Result<()> {
    imp::set_enabled(enabled)
}

// ---------------------------------------------------------------------------
// Platform implementation: Windows registry (HKCU Run key).
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use std::io;
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
    };

    /// Subkey path of the per-user Run key.
    const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    /// Encodes a Rust string as a NUL-terminated UTF-16 buffer for the wide
    /// (`*W`) registry APIs.
    fn wide(value: &str) -> Vec<u16> {
        std::ffi::OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// RAII guard that closes an opened registry key on drop.
    struct RegKey(HKEY);

    impl Drop for RegKey {
        fn drop(&mut self) {
            // SAFETY: `self.0` is a key handle returned by a successful open.
            unsafe {
                RegCloseKey(self.0);
            }
        }
    }

    /// Opens the Run key with the requested access rights.
    fn open_run_key(access: u32) -> io::Result<RegKey> {
        let subkey = wide(RUN_SUBKEY);
        let mut handle: HKEY = std::ptr::null_mut();
        // SAFETY: `subkey` is a valid NUL-terminated wide string for the call;
        // `handle` receives the opened key on success.
        let status =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, access, &mut handle) };
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        Ok(RegKey(handle))
    }

    /// Builds the command line registered under the Run key: the quoted path to
    /// the current executable followed by the auto-start flag.
    fn run_command() -> io::Result<String> {
        let exe = std::env::current_exe()?;
        Ok(format!("\"{}\" {}", exe.display(), super::AUTOSTART_FLAG))
    }

    pub fn is_enabled() -> bool {
        let Ok(key) = open_run_key(KEY_QUERY_VALUE) else {
            return false;
        };
        let name = wide(super::RUN_VALUE_NAME);
        // SAFETY: `name` is a valid NUL-terminated wide string; all output
        // pointers are null because we only probe for the value's existence.
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                name.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        status == ERROR_SUCCESS
    }

    pub fn set_enabled(enabled: bool) -> io::Result<()> {
        if enabled {
            enable()
        } else {
            disable()
        }
    }

    fn enable() -> io::Result<()> {
        let key = open_run_key(KEY_SET_VALUE)?;
        let name = wide(super::RUN_VALUE_NAME);
        let data = wide(&run_command()?);
        let byte_len = std::mem::size_of_val(data.as_slice()) as u32;
        // SAFETY: `name` is a valid NUL-terminated wide string; `data` points to
        // `byte_len` bytes of a NUL-terminated UTF-16 string, matching REG_SZ.
        let status = unsafe {
            RegSetValueExW(
                key.0,
                name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr() as *const u8,
                byte_len,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        Ok(())
    }

    fn disable() -> io::Result<()> {
        let key = open_run_key(KEY_SET_VALUE)?;
        let name = wide(super::RUN_VALUE_NAME);
        // SAFETY: `name` is a valid NUL-terminated wide string.
        let status = unsafe { RegDeleteValueW(key.0, name.as_ptr()) };
        // A missing value means it is already disabled, which is success.
        if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Platform implementation: non-Windows no-op fallback.
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
mod imp {
    use std::io;

    pub fn is_enabled() -> bool {
        false
    }

    pub fn set_enabled(_enabled: bool) -> io::Result<()> {
        Ok(())
    }
}
