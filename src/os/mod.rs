//! Platform-specific module for operating system features.

#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub mod unix;

/// Check if the current process has administrator privileges.
#[cfg(unix)]
pub fn has_admin_privileges() -> bool {
    // On Unix, check if EUID is 0 (root)
    nix::unistd::geteuid() == nix::unistd::Uid::from_raw(0)
}

/// Check if the current process has administrator privileges.
#[cfg(windows)]
pub fn has_admin_privileges() -> bool {
    windows::is_running_as_admin()
}

/// Request elevation and restart the current process with admin privileges.
#[cfg(unix)]
pub fn request_elevation(_args: &[String]) -> std::io::Result<()> {
    // On Unix, sudo can be used but it's not automatic
    // User needs to run with sudo manually
    Ok(())
}

/// Request elevation and restart the current process with admin privileges.
#[cfg(windows)]
pub fn request_elevation(args: &[String]) -> std::io::Result<()> {
    windows::run_as_admin(args)
}

/// Get the platform-specific symlink implementation function.
/// Returns None if the platform doesn't support the symlink operation directly.
#[cfg(windows)]
pub fn needs_elevation_for_symlink() -> bool {
    windows::symlink_needs_elevation()
}

#[cfg(unix)]
pub fn needs_elevation_for_symlink() -> bool {
    false
}
