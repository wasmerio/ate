ATE File System
===============

## What is ATE File System?

ATE File System is a distributed file system built on top of the ATE distributed
datastore. While the author is fully aware that the ecosystem of file-systems is
rather saturated this is a good problem to have in the grand scheme of things, this
particular varient has some unique qualities that differentiate it from others.

## What is ATE

[See here](https://github.com/john-sharratt/ate/blob/master/README.md)

## Summary

ATE File System uses FUSE to redirect file system commands to a user-space program that
responds to IO. This user-space program queries an in-memory distributed database that
is synchronized with many consumers and producers. Data is replicated, encrypted, signed
and multi-user safe.

Another key property of the ATE File System is that it is totally software defined which
allows materialized views of structured non-file based data model to also be represented
within the emulated sections of the file system.

Features:

- Very highly scalable (relative to other file systems)
- Low latency reads through local redo log replication
- Write through caching with distributed commits
- Distributed locking on files
- Fully encrypted files and metadata
- Quantum resistant encryption throughout
- Programmable API for emulated files

## High Level Design

    .--[   App  ]---. .--[   App  ]---. .--[   App  ]---.
    |               | |               | |               |
    |>local redo-log| |>local redo-log| |>local redo-log|
    |.-------------.| |.-------------.| |.-------------.|
    || Chain     1 || || Replica P1  || || Replica P1  ||
    ||             || || Chain     2 || || Replica P2  ||
    || Replica P3  || ||      ^      || || Chain     3 ||
    |*-------------*| |*------|------*| |*-------------*|
    |               |       subscribe                   
    |                \________|__________________________
    |                         |                          
    |  >local redo-log                                   
    |  >Crypto-Graph Materiaized View< (in memory)       
    |  .----------------------------------.      session 
    |  |             root                 |   .-----------.
    |  |              |                   |   |  -token   |
    |  |      dao----dao                  |---|  -claims  |
    |  |              \                   |   |  -keys    |
    |  |               dao                |   |  -timeout |
    |  |                                  |   *-----------*
       +----------------------------------+----------------+
       |                      atefs                        |
       +---------------------------------------------------+
       |                 fuse (/dev/fuse)                  |
       +---------------------------------------------------+
       |                   Linux Kernel                    |
       +---------------------------------------------------+
       |           Linux System Calls (e.g. read)          |
       +---------------------------------------------------+

## Installation

```sh
sudo apt install cargo make pkg-config libfuse-dev libfuse3-dev openssl libssl-dev
cargo install atefs
```

## Manual

```
atefs 1.3
John S. <johnathan.sharratt@gmail.com>

USAGE:
    atefs [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -d, --debug      Logs debug info to the console
        --dns-sec    Determines if ATE will use DNSSec or just plain DNS
    -h, --help       Prints help information
    -n, --no-auth    No authentication or passcode will be used to protect this file-system
        --no-ntp     No NTP server will be used to synchronize the time thus the server time will be
                     used instead
    -v, --verbose    Sets the level of log verbosity, can be used multiple times
    -V, --version    Prints version information

OPTIONS:
    -a, --auth <auth>
            URL where the user is authenticated [default: tcp://auth.tokera.com:5001/auth]

        --dns-server <dns-server>
            Address that DNS queries will be sent to [default: 8.8.8.8]

        --ntp-pool <ntp-pool>
            NTP server address that the file-system will synchronize with

        --ntp-port <ntp-port>
            NTP server port that the file-system will synchronize with

    -t, --token <token>
            Token used to access your encrypted file-system (if you do not supply a token then you
            will be prompted for a username and password)

        --token-path <token-path>
            Token file to read that holds a previously created token to be used to access your
            encrypted file-system (if you do not supply a token then you will be prompted for a
            username and password)

        --wire-encryption <wire-encryption>
            Indicates if ATE will use quantum resistant wire encryption (possible values are 128,
            192, 256). The default is not to use wire encryption meaning the encryption of the event
            data itself is what protects the data


SUBCOMMANDS:
    group    Groups are collections of users that share same remote file system
    help     Prints this message or the help of the given subcommand(s)
    mount    Mounts a local or remote file system
    token    Tokens are needed to mount file systems without prompting for credentials
    user     Users are needed to access any remote file systems

--------------------------------------------------------------------------

Users are needed to access any remote file systems

USAGE:
    atefs user <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    create     Creates a new user and generates login credentials
    details    Returns all the details about a specific user
    help       Prints this message or the help of the given subcommand(s)

--------------------------------------------------------------------------

Groups are collections of users that share same remote file system

USAGE:
    atefs group <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    add-user       Adds another user to an existing group
    create         Creates a new group
    details        Display the details about a particular group (token is required to see role
                   membership)
    help           Prints this message or the help of the given subcommand(s)
    remove-user    Removes a user from an existing group

--------------------------------------------------------------------------

Tokens are needed to mount file systems without prompting for credentials

USAGE:
    atefs token <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    gather      Gather the permissions needed to access a specific group into the token using
                either another supplied token or the prompted credentials
    generate    Generate a token with normal permissions from the supplied username and password
    help        Prints this message or the help of the given subcommand(s)
    sudo        Generate a token with extra permissions with elevated rights to modify groups
                and other higher risk actions

--------------------------------------------------------------------------

Mounts a local or remote file system

USAGE:
    atefs mount [FLAGS] [OPTIONS] <mount-path> [ARGS]

ARGS:
    <mount-path>    Path to directory that the file system will be mounted at
    <remote>        URL where the data is remotely stored on a distributed commit log (e.g.
                    tcp://ate.tokera.com/). If this URL is not specified then data will only be
                    stored locally
    <log-path>      (Optional) Location of the local persistent redo log (e.g. ~/ate/fs)

FLAGS:
        --allow-other        Allow other users on the machine to have access to this file system
        --allow-root         Allow the root user to have access to this file system
        --compact-now        Forces the compaction of the local redo-log before it streams in the
                             latest values
    -h, --help               Prints help information
    -i, --impersonate-uid    For files and directories that the authenticated user owns, translate
                             the UID and GID to the local machine ids instead of the global ones
        --non-empty          Allow fuse filesystem mount on a non-empty directory, default is not
                             allowed
    -r, --read-only          Mount the file system in readonly mode (`ro` mount option), default is
                             disable
        --temp               Local redo log file will be deleted when the file system is unmounted,
                             remotely stored data on any distributed commit log will be persisted.
                             Effectively this setting only uses the local disk as a cache of the
                             redo-log while it's being used
    -V, --version            Prints version information
    -w, --write-back         Enable write back cache for buffered writes, default is disable

OPTIONS:
        --compact-mode <compact-mode>
            Mode that the compaction will run under (valid modes are 'never', 'modified', 'timer',
            'factor', 'size', 'factor-or-timer', 'size-or-timer') [default: factor-or-timer]

        --compact-threshold-factor <compact-threshold-factor>
            Factor growth in the log file which will trigger compaction - this
            argument is ignored if you select a compact_mode that has no growth trigger [default:
            0.4]

        --compact-threshold-size <compact-threshold-size>
            Size of growth in bytes in the log file which will trigger compaction (default: 100MB) -
            this argument is ignored if you select a compact_mode that has no growth trigger
            [default: 104857600]

        --compact-timer <compact-timer>
            Time in seconds between compactions of the log file (default: 1 hour) - this argument is
            ignored if you select a compact_mode that has no timer [default: 3600]

        --configured-for <configured-for>
            Configure the log file for <raw>, <barebone>, <speed>, <compatibility>, <balanced> or
            <security> [default: speed]

        --data-format <data-format>
            Format of the data in the log file as <bincode>, <json> or <mpack> [default: bincode]

    -g, --gid <gid>
            GID of the group that this file system will be mounted as

        --meta-format <meta-format>
            Format of the metadata in the log file as <bincode>, <json> or <mpack> [default:
            bincode]

    -p, --passcode <passcode>
            User supplied passcode that will be used to encrypt the contents of this file-system
            instead of using an authentication. Note that this can 'not' be used as combination with
            a strong authentication system and hence implicitely implies the 'no-auth' option as
            well

        --recovery-mode <recovery-mode>
            Determines how the file-system will react while it is nominal and when it is recovering
            from a communication failure (valid options are 'async', 'readonly-async', 'readonly-
            sync' or 'sync') [default: readonly-async]

    -u, --uid <uid>
            UID of the user that this file system will be mounted as

```

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)