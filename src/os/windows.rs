//! Windows-specific operating system features.

use std::io;
use std::os::windows::ffi::OsStrExt;
use winapi::um::shellapi::ShellExecuteW;
use winapi::um::winuser::SW_SHOW;
use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
use winapi::um::handleapi::CloseHandle;
use winapi::um::winnt::{HANDLE, TOKEN_ELEVATION, TokenElevation, TOKEN_QUERY};
use winapi::ctypes::c_void;

/// Check if the current process is running with administrator privileges.
pub fn is_running_as_admin() -> bool {
    let mut token_handle: HANDLE = std::ptr::null_mut();
    let mut is_admin = false;

    // Try to open the process token
    let success = unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY,
            &mut token_handle,
        )
    };

    if success != 0 && !token_handle.is_null() {
        let mut token_info: TOKEN_ELEVATION = unsafe { std::mem::zeroed() };
        let mut return_length: u32 = 0;

        let query_success = unsafe {
            winapi::um::securitybaseapi::GetTokenInformation(
                token_handle,
                TokenElevation,
                &mut token_info as *mut _ as *mut c_void,
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut return_length,
            )
        };

        unsafe {
            CloseHandle(token_handle);
        }

        if query_success != 0 {
            is_admin = token_info.TokenIsElevated != 0;
        }
    }

    is_admin
}

/// Check if creating a symbolic link on Windows requires elevation.
/// Windows Symlink requires admin privileges unless Developer mode is enabled.
pub fn symlink_needs_elevation() -> bool {
    !is_running_as_admin()
}

/// Run the current executable with administrator privileges.
pub fn run_as_admin(args: &[String]) -> io::Result<()> {
    let exe_path = std::env::current_exe()?;

    // Build the command line
    let mut cmdline = String::new();
    for arg in args {
        if !cmdline.is_empty() {
            cmdline.push(' ');
        }
        if arg.contains(' ') || arg.contains('"') {
            cmdline.push('"');
            cmdline.push_str(&arg.replace('"', "\"\""));
            cmdline.push('"');
        } else {
            cmdline.push_str(arg);
        }
    }

    let operation: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();
    let exe_path_utf16: Vec<u16> = exe_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let params_utf16: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            exe_path_utf16.as_ptr(),
            if cmdline.is_empty() { std::ptr::null() } else { params_utf16.as_ptr() },
            std::ptr::null(),
            SW_SHOW,
        )
    };

    // ShellExecuteW returns a value > 32 on success
    if result as i32 > 32 {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Failed to request elevation"))
    }
}

/// Re-execute the current process with elevated privileges for symlink operations.
pub fn elevate_for_symlink() -> io::Result<()> {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|os| os.into_string().unwrap_or_default())
        .filter(|s| !s.is_empty())
        .collect();

    let mut elevated_args = vec!["--elevated-for-symlink".to_string()];
    elevated_args.extend(args);

    run_as_admin(&elevated_args)
}

/// Check if this process was started due to an elevation request.
pub fn was_started_for_elevation() -> bool {
    std::env::args().any(|arg| arg == "--elevated-for-symlink")
}
