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

    pub const WELCOME: &'static str = r#"\x1B[0m
\x1B[33;1m|                           /\       -X-     + |\x1B[1;34m                 ww              \x1B[37;1m  Tokera WASM.sh v1.0\r
\x1B[33;1m|              +           /  \_               |\x1B[1;34m                 wwwww           \x1B[37;1m  Powered by Wasmer and ATE\r
\x1B[33;1m|       /\               _/ ^ / \  _           |\x1B[1;34m          ww     wwwwww  w       \x1B[37;1m  \r
\x1B[33;1m|    /\/  \      +      |   _/ ` \/ \    +     |\x1B[1;34m          wwwww      wwwwwwwww   \x1B[0m  QUICK START:\r
\x1B[33;1m|   //  `  \            |  / __   \ `\    _    |\x1B[1;34m  ww      wwwwww  w     wwwwwww  \x1B[0m      • WAPM commands -> wapm\r
\x1B[33;1m|  //  .    \         /^|_/ / .\ . \ ~\  / \   |\x1B[1;34m  wwwww      wwwwwwwwww   wwwww  \x1B[0m      • Tokera commands -> tok\r
\x1B[33;1m| //     ^ . \_______| /   / _  \   \  \_| |__/|\x1B[1;34m  wwwwww  w      wwwwwww  wwwww  \x1B[0m      • Core utils: coreutils -> ls\r
\x1B[33;1m|     _____     _                        | |   |\x1B[1;34m  wwwwwwwwwwwwww   wwwww  wwwww  \x1B[0m      • Pipe -> echo blah | cowsay\r
\x1B[33;1m|    |_   _|__ | | _____ ___ __ _*      /| |\  |\x1B[1;34m  wwwwwwwwwwwwww   wwwww  wwwww  \x1B[0m      • Run a JS Engine -> qjs\r
\x1B[33;1m|      | |/ _ \| |/ / _ \  _/ _  |* ,  | |_| | |\x1B[1;34m  wwwwwwwwwwwwww   wwwww  wwwww  \x1B[0m      • Access a wallet -> wallet\r
\x1B[33;1m|      | | (_) |   <  __/ || (_| |     |/ " \| |\x1B[1;34m  wwwwwwwwwwwwww   wwwww   wwww  \x1B[0m  \r
\x1B[33;1m| ~    |_|\___/|_|\_\___|_| \__/_|       """   |\x1B[1;34m  wwwwwwwwwwwwwww  wwwww         \x1B[0m  MORE INFO:\r
\x1B[33;1m|______________________________________________|\x1B[1;34m     wwwwwwwwwwww   wwww         \x1B[0m      • Usage Information: help\r
\x1B[37;1m|   Tokera v1.0 Copyright (C) 2016 Tokera Ltd  |\x1B[1;34m         wwwwwwww                \x1B[0m      • About Tokera: about\r
\x1B[37;1m|              ( www.tokera.com )              |\x1B[1;34m             wwww                \x1B[0m  \r
\x1B[0m\r\n"#;

    pub const ABOUT: &'static str = r#"# Tokera WASM.sh

The Tokera WASM shell is an browser based operating system that integrates
with the WebAssembly community to assembly and build micro-applications.

Including:
- MemFS file system with mount points
- stdin, stdout, stderr and tty support
- Private file system space per process.
- Full support for piping and TTY.
- Fully Multi-threading.
- Support for basic bash commands.
- Environment variables.

Tokera is built on to of ATE and Wasmer:

https://github.com/john-sharratt/ate
https://github.com/wasmerio/wasmer

Visit us at:
https://www.tokera.com
"#;

    pub const HELP: &'static str = r#"# Tokera WASM.sh

## The Shell

The Tokera WASM shell is an browser based operating system that integrates
with the WebAssembly community to assembly and build micro-applications.

Including:
- MemFS file system with mount points
- stdin, stdout, stderr and tty support
- Private file system space per process.
- Full support for piping and TTY.
- Fully Multi-threading.
- Support for basic bash commands.
- Environment variables.

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
    truncate, tsort, unexpand, uniq, unlink, wc, yes
    
## tokera commands:

    contract    Contracts represent all the subscriptions you have made to specific services you
                personally consume or a group consume that you act on your authority on behalf
                of. This sub-menu allows you to perform actions such as cancel said contracts
    domain      Domain groups are collections of users that share something together in
                association with an internet domain name. Every group has a built in wallet(s)
                that you can use instead of a personal wallet. In order to claim a domain group
                you will need DNS access to an owned internet domain that can be validated
    help        Print this message or the help of the given subcommand(s)
    login       Login to an account and store the token locally for reuse
    logout      Logout of the account by deleting the local token
    service     Services offered by Tokera (and other 3rd parties) are accessible via this sub
                command menu, including viewing the available services and subscribing to them
    user        Users are personal accounts and services that have an authentication context.
                Every user comes with a personal wallet that can hold commodities
    wallet      Wallets are directly attached to groups and users - they hold a balance, store
                transaction history and facilitate transfers, deposits and withdraws"#;
}