# Tokera WASM.sh

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

    add          Add packages to the manifest without installing
    bin          Get the .bin dir path
    config       Config related subcommands
    help         Prints this message or the help of the given subcommand(s)
    init         Set up current directory for use with wapm
    install      Install a package
    remove       Remove packages from the manifest
    uninstall    Uninstall a package
    validate     Check if a directory or tar.gz is a valid wapm package

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
                transaction history and facilitate transfers, deposits and withdraws