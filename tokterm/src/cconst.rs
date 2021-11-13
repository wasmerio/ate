#![allow(dead_code)]
use super::tty::Tty;
use super::console::Console;

impl Console
{
    pub const TERM_KEY_ENTER: u32 = 13;
    pub const TERM_KEY_BACKSPACE: u32 = 8;
    pub const TERM_KEY_INSERT: u32 = 45;
    pub const TERM_KEY_DEL: u32 = 46;
    pub const TERM_KEY_TAB: u32 = 9;
    pub const TERM_KEY_HOME: u32 = 36;
    pub const TERM_KEY_END: u32 = 35;
    pub const TERM_KEY_PAGE_UP: u32 = 33;
    pub const TERM_KEY_PAGE_DOWN: u32 = 34;
    pub const TERM_KEY_LEFT_ARROW: u32 = 37;
    pub const TERM_KEY_UP_ARROW: u32 = 38;
    pub const TERM_KEY_RIGHT_ARROW: u32 = 39;
    pub const TERM_KEY_DOWN_ARROW: u32 = 40;
    pub const TERM_KEY_C: u32 = 67;
    pub const TERM_KEY_L: u32 = 76;
    pub const TERM_KEY_F1: u32 = 112;
    pub const TERM_KEY_F2: u32 = 113;
    pub const TERM_KEY_F3: u32 = 114;
    pub const TERM_KEY_F4: u32 = 115;
    pub const TERM_KEY_F5: u32 = 116;
    pub const TERM_KEY_F6: u32 = 117;
    pub const TERM_KEY_F7: u32 = 118;
    pub const TERM_KEY_F8: u32 = 119;
    pub const TERM_KEY_F9: u32 = 120;
    pub const TERM_KEY_F10: u32 = 121;
    pub const TERM_KEY_F11: u32 = 122;
    pub const TERM_KEY_F12: u32 = 123;
}

impl Tty
{
    pub const TERM_CURSOR_UP: &'static str = "\x1b[A";
    pub const TERM_CURSOR_DOWN: &'static str = "\x1b[B";
    pub const TERM_CURSOR_RIGHT: &'static str = "\x1b[C";
    pub const TERM_CURSOR_LEFT: &'static str = "\x1b[D";

    pub const TERM_DELETE_LINE: &'static str = "\x1b[2K\r";
    pub const TERM_DELETE_RIGHT: &'static str = "\x1b[0K\r";
    pub const TERM_DELETE_LEFT: &'static str = "\x1b[1K\r";
    pub const TERM_DELETE_BELOW: &'static str = "\x1b[0J\r";
    pub const TERM_DELETE_ABOVE: &'static str = "\x1b[1J\r";
    pub const TERM_DELETE_ALL: &'static str = "\x1b[2J\r";
    pub const TERM_DELETE_SAVED: &'static str = "\x1b[3J\r";

    pub const TERM_CURSOR_SAVE: &'static str = "\x1b[s";
    pub const TERM_CURSOR_RESTORE: &'static str = "\x1b[u";

    pub const TERM_WRAPAROUND: &'static str = "\x1b[?7h";
    pub const TERM_REVERSE_WRAPAROUND: &'static str = "\x1b[?45h";

    pub const TERM_NO_WRAPAROUND: &'static str = "\x1b[?7l";
    pub const TERM_NO_REVERSE_WRAPAROUND: &'static str = "\x1b[?45l";

    pub const COL_RESET: &'static str ="\x1B[0m";
    pub const COL_BLACK: &'static str ="\x1B[0;30m";
    pub const COL_GRAY: &'static str ="\x1B[1;30m";
    pub const COL_RED: &'static str ="\x1B[0;31m";
    pub const COL_LIGHT_RED: &'static str ="\x1B[1;31m";
    pub const COL_GREEN: &'static str ="\x1B[0;32m";
    pub const COL_LIGHT_GREEN: &'static str ="\x1B[1;32m";
    pub const COL_BROWN: &'static str ="\x1B[0;33m";
    pub const COL_YELLOW: &'static str ="\x1B[1;33m";
    pub const COL_BLUE: &'static str ="\x1B[0;34m";
    pub const COL_LIGHT_BLUE: &'static str ="\x1B[1;34m";
    pub const COL_PURPLE: &'static str ="\x1B[0;35m";
    pub const COL_LIGHT_PURPLE: &'static str ="\x1B[1;35m";
    pub const COL_CYAN: &'static str ="\x1B[0;36m";
    pub const COL_LIGHT_CYAN: &'static str ="\x1B[1;36m";
    pub const COL_LIGHT_GRAY: &'static str ="\x1B[0;37m";
    pub const COL_WHITE: &'static str ="\x1B[1;37m";

    pub const WELCOME: &'static str = r#""#;

    pub const ABOUT_TOKERA: &'static str = r#"#
The WebAssembly Shell is built with Wasmer and ATE:

    https://github.com/john-sharratt/ate
    https://github.com/wasmerio/wasmer

Visit us at:
https://www.tokera.com
"#;

   pub const ABOUT_WASMER: &'static str = r#"# Wasmer

Wasmer is a fast and secure WebAssembly runtime that enables super
lightweight containers to run anywhere: from Desktop to the Cloud, Edge and
IoT devices.

Features:
• Secure by default. No file, network, or environment access, unless
  explicitly enabled.
• Supports WASI and Emscripten out of the box.
• Fast. Run WebAssembly at near-native speeds.
• Embeddable in multiple programming languages
• Compliant with latest WebAssembly Proposals (SIMD, Reference Types,
  Threads, ...)"#;

    pub const HELP: &'static str = r#"#

## wapm commands

    list                           List the currently installed packages and their commands
    install                        Install a package from Wapm
    upload                         Install a local Wasm module
    uninstall                      Uninstall a package

## coreutil commands:

    arch, base32, base64, basename, cat, cksum, comm, cp, csplit, cut,
    date, dircolors, dirname, echo, env, expand, factor, false, fmt, fold,
    hashsum, head, join, link, ln, ls, md5sum, mkdir, mktemp, mv, nl, nproc,
    numfmt, od, paste, printenv, printf, ptx, pwd, readlink, realpath,
    relpath, rm, rmdir, seq, sha1sum, sha224sum, sha256sum, sha3-224sum,
    sha3-256sum, sha3-384sum, sha3-512sum, sha384sum, sha3sum, sha512sum,
    shake128sum, shake256sum, shred, shuf, sleep, sum, tee, touch, tr, true,
    truncate, tsort, unexpand, uniq, unlink, wc, yes"#;
}