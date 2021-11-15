use libc::{self, c_char, c_int};
use std::ffi::{CStr, CString};
use std::io;
use std::path::Path;

extern "C" {
    // *_compat25 functions were introduced in FUSE 2.6 when function signatures changed.
    // Therefore, the minimum version requirement for *_compat25 functions is libfuse-2.6.0.
    pub fn fuse_unmount_compat22(mountpoint: *const c_char);
}

/// Unmount an arbitrary mount point
pub fn unmount(mountpoint: &Path) -> io::Result<()> {
    // fuse_unmount_compat22 unfortunately doesn't return a status. Additionally,
    // it attempts to call realpath, which in turn calls into the filesystem. So
    // if the filesystem returns an error, the unmount does not take place, with
    // no indication of the error available to the caller. So we call unmount
    // directly, which is what osxfuse does anyway, since we already converted
    // to the real path when we first mounted.

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "bitrig",
        target_os = "netbsd"
    ))]
    #[inline]
    fn libc_umount(mnt: &CStr) -> c_int {
        unsafe { libc::unmount(mnt.as_ptr(), 0) }
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "bitrig",
        target_os = "netbsd"
    )))]
    #[inline]
    fn libc_umount(mnt: &CStr) -> c_int {
        use std::io::ErrorKind::PermissionDenied;

        let rc = unsafe { libc::umount(mnt.as_ptr()) };
        if rc < 0 && io::Error::last_os_error().kind() == PermissionDenied {
            // Linux always returns EPERM for non-root users.  We have to let the
            // library go through the setuid-root "fusermount -u" to unmount.
            unsafe {
                fuse_unmount_compat22(mnt.as_ptr());
            }
            0
        } else {
            rc
        }
    }

    let mnt = CString::new(
        mountpoint
            .to_path_buf()
            .into_os_string()
            .into_string()
            .unwrap(),
    )?;
    let rc = libc_umount(&mnt);
    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
