ATE Authentication Model
=======================

## What is ATE Authentication Model?

The ATE authentication model is a set of data objects and business logic that allow
providers to create authentication systems that are naturally integrated with ATE
in an abstract way.

## What is ATE

[See here](../README.md)

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)

## Authentication Server Usage

```
USAGE:
    auth-server [FLAGS] <SUBCOMMAND>

FLAGS:
    -d, --debug      Logs debug info to the console
    -h, --help       Prints help information
    -v, --verbose    Sets the level of log verbosity, can be used multiple times
    -V, --version    Prints version information

SUBCOMMANDS:
    generate    Generates the secret key that helps protect key operations like creating users
                and resetting passwords
    help        Prints this message or the help of the given subcommand(s)
    run         Runs the login server

--------------------------------------------------------------------------

Generates the secret key that helps protect key operations like creating users and resetting
passwords

USAGE:
    auth-server generate [OPTIONS] [key-path]

ARGS:
    <key-path>    Path to the secret key [default: ~/ate/auth.key]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -s, --strength <strength>    Strength of the key that will be generated [default: 256]

--------------------------------------------------------------------------

Runs the login authentication and authorization server

USAGE:
    auth-server run [OPTIONS] [ARGS]

ARGS:
    <logs-path>    Path to the log files where all the authentication data is stored [default:
                   ~/ate/auth]
    <key-path>     Path to the secret key that helps protect key operations like creating users
                   and resetting passwords [default: ~/ate/auth.key]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -l, --listen <listen>    IP address that the authentication server will isten on [default:
                             0.0.0.0]
    -p, --port <port>        Port that the authentication server will listen on [default: 5001]
```

## Authentication Tools Usage

```
USAGE:
    auth-tools [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -d, --debug      Logs debug info to the console
    -h, --help       Prints help information
    -v, --verbose    Sets the level of log verbosity, can be used multiple times
    -V, --version    Prints version information

OPTIONS:
    -a, --auth <auth>                URL where the user is authenticated [default:
                                     ws://tokera.com/auth]
    -t, --token <token>              Token used to access your encrypted file-system (if you do not
                                     supply a token then you will be prompted for a username and
                                     password)
        --token-path <token-path>    Token file to read that holds a previously created token to be
                                     used to access your encrypted file-system (if you do not supply
                                     a token then you will be prompted for a username and password)

SUBCOMMANDS:
    group    Groups are collections of users that share something together
    help     Prints this message or the help of the given subcommand(s)
    token    Tokens are stored authentication and authorization secrets used by other processes
    user     Users are personal accounts and services that have an authentication context

--------------------------------------------------------------------------

Users are personal accounts and services that have an authentication context

USAGE:
    auth-tools user <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    create     Creates a new user and generates login credentials
    details    Returns all the details about a specific user
    help       Prints this message or the help of the given subcommand(s)

--------------------------------------------------------------------------

Groups are collections of users that share something together

USAGE:
    auth-tools group <SUBCOMMAND>

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

Tokens are stored authentication and authorization secrets used by other processes

USAGE:
    auth-tools token <SUBCOMMAND>

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
```