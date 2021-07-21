ATE Database
===============

## What is ATE Database?

ATE Database is a distributed redo log server that the ATE projects can use to
remotely store their redo logs. ATE is designed to connect to remotely hosted
repositories of state for to meet integrity and availability non-functional
requirements.

## What is ATE

[See here](../README.md)

## Summary

ATE Database runs a server daemon that listens for connections from clients and serves
as a distributed redo log.

Other projects use this backend for persistent storage - projects such as

- [tokfs](../tokfs/README.md)

## How it works

- AteDB will store its log files on a local file-system using a naming convention that
  follows the URL path of connecting clients.
- Log files are loaded and updated on-demand as consumers connect, feed events and disconnect
- AteDB will listen on a TCP port (default: 5000) with both IPv4 and IPv6 protocols unless
  otherwise specified
- When a client attempts to access a chain-key that does not exist AteDB will create a new
  chain and associated log files.
- When AteDB creates new chains it will will query an authentication server for a public key
  of who should have access to write root events into the chain based on a naming convention.  
  e.g. ws://server.com/db/domain.com/name/mydb => mydb chain for user with email name@domain.com.
- If you specify the --no-auth command then chains will be created without making any authentication
  checks which means anyone can create new chains on a first-come-first-served basis and no authentcation
  server is required.

By default all wire messages are clear text but the events that are confidential are encrypted
thus giving a good balance between security and speed however you may also increase the security
of databases by using double-encryption. For instance if you specify --wire-encryption 128 then
AteDB will negotiate AES 128bit symetric keys using a quantum resistant key exchange that supports
perfect forward secrecy symentics.

There are two modes that AteDB can run in which have very different characteristics of trust and speed.

### Centralized (default)

Repreresents a **centralized** trust model meaning that the communication link itself with a central
server becomes a trusted conversation. When the server or client sends its events it will only
provide proof of ownership of authorization to write 'once' per connection. This means that
writes will be much faster as they only need to compute an asymetric signature once for each
key that is used however it also means that you are trusting that the server is doing its job
properly and has not been compromised by an attacker who has added their own events.

When running in centralized mode it is highly recommended you use wire-encryption to prevent an
attacker from injecting fake events into your communication channel. Use the --wire-encryption
parameter to enable this. For this reason AteDB will default to using 128bit wire encryption
when running in centralized mode which can be disabled using the --no-wire-encryption parameter.

### Distributed

Represents **distributed** trust model where the only thing clients and servers really trust is
the signatures of each individual event. In the scenario that a server is compromised it is not
possible for an attacker to inject their own events into the chains as they do not have ownership
of the signing private keys. This mode has a fairly significant impact on write operations as it
means events need a asynmetric signature computed on all writes, batching attempts to minimize this
cost however if writes are individually committed then a worst case scenario of every IO equals
a signature will eventulate.

In this mode it is not nessasary to also run wire encryption as all events that require
confidentiallity and integrity are protected individually however one can reduce the changes of
side channel attacks and denial of service risks through double-encryption.

When running in 'distributed' mode the database will not make an authentication server requests
as there is nothing to gain from this or to validate using. All integrity is provided as a part
of the events themselves. Hence in this mode the --auth setting has no effect

## QuickStart

```sh
# Installation and upgrade AteDB and auth-server
apt install cargo make pkg-config openssl libssl-dev
cargo install atedb
cargo install auth-server
```

```sh
# Launch AteDB with all the defaults which is a good balance of security, performance
# and simplfied setup. This instance will use the default authentication when it creates
# new chains setting the root write key to that of the owner. The authentication server
# that is queried will default to ws://tokera.com/auth.
# The instance will listen on all ports and all network addresses.
atedb solo
```

```sh
# Starts a single instance database on the default port (5000) . This mode will use a
# distributed trust model which means it does not need an authentication server and will
# default to 128bit AES wire encryption
atedb --trust distributed solo
```

```sh
# Launch AteDB with the added protection of double-encryption with all communication to this
# server protected by 256bit AES encryption using quantum resistant key exchange. This mode
# of operation is the most secure but also the least performant.
atedb --wire-encryption 256 --trust distributed solo
```

```sh
# Load and store log files in a different path than the default of ~/ate
atedb solo ~/another-path/
```

```sh
# Starts AteDB without an authentication server which will listen on a localhost address for connections.
# Wire encryption is disabled even while this instance is running as a centralized trust mode as there
# is a low risk attackers could reach the loopback device.
atedb --no-auth --no-wire-encryption solo -l 127.0.0.1 -p 5555
```

```sh
# Starts AteDB using a different DNS server and authentication address that is hosted locally
# Also by specifying that we listen on address 0.0.0.0 we purposely limited ourselves to IPv4
auth-server run -l 0.0.0.0 -p 5555
atedb --dns 8.8.4.4 --auth ws://localhost:5555/auth solo -l 0.0.0.0
```

```sh
# Show log information of the running AteDB
RUST_LOG=info atedb solo
```

## Manual

```
USAGE:
    atedb [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -d, --debug                 Logs debug info to the console
        --dns-sec               Determines if ATE will use DNSSec or just plain DNS
    -h, --help                  Prints help information
        --no-auth               Indicates no authentication server will be used meaning all new
                                chains created by clients allow anyone to write new root nodes
        --no-wire-encryption    Disbles wire encryption which would otherwise be turned on when
                                running in 'centralized' mode
    -v, --verbose               Sets the level of log verbosity, can be used multiple times
    -V, --version               Prints version information

OPTIONS:
    -a, --auth <auth>
            URL where the user is authenticated [default: ws://tokera.com/auth]

        --dns-server <dns-server>
            Address that DNS queries will be sent to [default: 8.8.8.8]

    -t, --trust <trust>
            Trust mode that the database server will run under - valid values are either
            'distributed' or 'centralized'. When running in 'distributed' mode the server itself
            does not need to be trusted in order to trust the data it holds however it has a
            significant performance impact on write operations while the 'centralized' mode gives
            much higher performance but the server needs to be protected [default: centralized]

        --wire-encryption <wire-encryption>
            Indicates if ATE will use quantum resistant wire encryption (possible values are 128,
            192, 256). When running in 'centralized' mode wire encryption will default to 128bit
            however when running in 'distributed' mode wire encryption will default to off unless
            explicitly turned on


SUBCOMMANDS:
    help    Prints this message or the help of the given subcommand(s)
    solo    Runs a solo ATE database and listens for connections from clients

--------------------------------------------------------------------------

Runs a solo ATE database and listens for connections from clients

USAGE:
    atedb solo [OPTIONS] [logs-path]

ARGS:
    <logs-path>    Path to the log files where all the file system data is stored [default:
                   /opt/ate]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --compact-mode <compact-mode>
            Mode that the compaction will run under (valid modes are 'never', 'modified', 'timer',
            'factor', 'size', 'factor-or-timer', 'size-or-timer') [default: growth-or-timer]

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

    -l, --listen <listen>
            IP address that the database server will isten on [default: 0.0.0.0]

    -p, --port <port>
            Port that the database server will listen on [default: 5000]


```

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)