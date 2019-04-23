package com.tokera.ate.enumerations;

import com.tokera.ate.units.LinuxError;

public class LinuxErrors {
    public static final @LinuxError String OK ="0";        // Everything is OK
    public static final @LinuxError String EPERM = "1";    // Operation not permitted
    public static final @LinuxError String ENOENT = "2";   // No such file or directory
    public static final @LinuxError String ESRCH = "3";    // No such process
    public static final @LinuxError String EINTR = "4";    // Interrupted system call
    public static final @LinuxError String EIO = "5";      // I/O Error
    public static final @LinuxError String ENXIO = "6";    // No such device or address
    public static final @LinuxError String E2BIG = "7";    // Argument list too long
    public static final @LinuxError String ENOEXEC = "8";  // Exec format error
    public static final @LinuxError String EBADF = "9";    // Bad file number
    public static final @LinuxError String ECHILD = "10";  // No child processes
    public static final @LinuxError String EAGAIN = "11";  // Try again
    public static final @LinuxError String ENOMEM = "12";  // Out of memory
    public static final @LinuxError String EACCES = "13";  // Permission denied
    public static final @LinuxError String EFAULT = "14";  // Bad address
    public static final @LinuxError String ENOTBLK = "15"; // Block device required
    public static final @LinuxError String EBUSY = "16";   // Device or resource busy
    public static final @LinuxError String EEXIST = "17";  // File exists
    public static final @LinuxError String EXDEV = "18";   // Cross-device link
    public static final @LinuxError String ENODEV = "19";  // No such device
    public static final @LinuxError String ENOTDIR = "20"; // Not a directory
    public static final @LinuxError String EISDIR = "21";  // Is a directory
    public static final @LinuxError String EINVAL = "22";  // Invalid argument
    public static final @LinuxError String ENFILE = "23";  // File table overflow;
    public static final @LinuxError String EMFILE = "24";  // Too many open files
    public static final @LinuxError String ENOTTY = "25";  // Not a typewriter
    public static final @LinuxError String ETXTBSY = "26"; // Text file busy
    public static final @LinuxError String EFBIG = "27";   // File too large
    public static final @LinuxError String ENOSPC = "28";  // No space left on device
    public static final @LinuxError String ESPIPE = "29";  // Illegal seek
    public static final @LinuxError String EROFS = "30";   // Read-only file system
    public static final @LinuxError String EMLINK = "31";  // Too many links
    public static final @LinuxError String EPIPE = "32";   // Broken pipe
    public static final @LinuxError String EDOM = "33";    // Math argument out of domain of func
    public static final @LinuxError String ERANGE = "34";   // Math result not representable
    public static final @LinuxError String EDEADLK = "35";  // Resource deadlock would occur
    public static final @LinuxError String ENAMETOOLONG = "36";    // File name too long
    public static final @LinuxError String ENOLCK = "37";  // No record locks available
    public static final @LinuxError String ENOSYS = "38";  // Function not implemented
    public static final @LinuxError String ENOTEMPTY = "39";   // Directory not empty
    public static final @LinuxError String ELOOP = "40";   // Too many symbolic links encountered
}
