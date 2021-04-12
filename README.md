ATE
===

## Navigation

- [Executive Summary](README.md)
- [User Guide for ATE](doc/guide.md)
- [Technical Design of ATE](doc/design.md)
- [Component Design of ATE](doc/components.md)

## What is ATE?

...is it a [NoSQL](https://en.wikipedia.org/wiki/NoSQL) database?  
...is it a [distributed redo log](https://en.wikipedia.org/wiki/Redo_log)?  
...is it a [event BUS](https://en.wikipedia.org/wiki/Publish%E2%80%93subscribe_pattern)?  
...is it a [API framework](https://en.wikipedia.org/wiki/Command_pattern)?  
...is it a [distributed queue](https://en.wikipedia.org/wiki/Message_queue)?  
...is it a [distributed cache](https://en.wikipedia.org/wiki/Distributed_cache)?  
...is it a secure encrypted vault?  
...is it a [quantum resistant](https://en.wikipedia.org/wiki/NIST_Post-Quantum_Cryptography_Competition) communication framework?  
...is it a [WORM](https://en.wikipedia.org/wiki/Write_once_read_many) archive solution?  
    
ATE is all these things and none of them; it is [unique way](doc/design.md) to work with distributed
data that can easily implement all of the above use cases - take a look at the [examples](#examples)
for how you can achieve them.

## Why the name?

The origin of the word "mutate" is the latin word '-ate':  
https://www.dictionary.com/browse/mutate

## Summary

ATE is a distributed immutable data store and built in memory based materialized
view with strong encryption and authentication.

What does that mean?

This library is a way of working with data in modern distributed computing.
* ...data is persisted to a distributed commit log.
* ...partitions are divided into chains that shard data into physical domains.
* ...streaming of data to the application occurs on demand during method invocation.
* ...each chain is a crypto-graph with unique asymmetric keys at differentiating nodes.
* ...the root of the chain-of-trust that validates the crypto-graph through various plugins.
* ...strong authentication and authorized is by design built into the data model.
* ...encryption is highly resistant to quantum attacks and uses fine-grained tenant keys.
* ...all this is integrated into a shared-nothing highly portable executable.

## Changelog

```
0.4.0 - Passed the milestone of a fully authenticated and encrypted remote database
        + Ate now has a fully functional command pattern that allows for RPC
        + Authentication server with multi-factor auth is functional
        + AteDB integrated with the authentication so all chains are owned by a user
        + AteFS upgraded so that it supports full authentication and confidentiality
0.3.5 - Finished the basics of the authentication server logic
0.3.4 - Added error handling to the invocation function and fixed some serious bugs
0.3.3 - Slight refactoring in order to improve the service invocation functionality
0.3.2 - Added beta service invocation functionality - see the new example
0.3.1 - AteDB is now functional allowing AteFS to store its files in the replicated
        redo log, also this helped find and solve a whole bunch of bugs.
0.3.0 - New API changes to cater for future releases with many fixes for critical bugs
        + Improvements to the log file format to support forward compatibility
        + Added wire encryption as another layer of defence using perfect-forward-secrecy
        + Modified the hash routines to reduce changes of collisions on key generation
        + Many bug fixes and performance optimizations
0.2.* - Alpha version with API changes
0.1.* - Alpha version with basic functionality and free bugs
```

## Todo

```
1.0.* - First release of a production grade version of ATE with AteFS using
        Tokera as authentication
0.*.* - Estimated 1-month of work before enough bugs are solved and the API
        can be fully stabilised
0.4.0 - Remaining things to do...
        + Fix the unit tests
        + Test why the performance is poor when writing remotely and tweak it
        + Implement logic that will resend partially sent local logs upon restart
        + Create a faster method of signing messages which is slightly less secure
          but much better performance
        + AteAuth requires account access rights that also get added to the token
        + Connect up the 'chmod' commands to real commands in AteAuth
0.4.1   + Create the ability to create a public file system
        + Modify AteFS so that it creates nodes in a global file system tree for
          everyone in the world to see (but not actually have access to)
```

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
    |  +----------------------------------+----------------+
    |  |                                                   |

## Quick Start

Add ate, serde and tokio to your dependency list in Cargo.toml

```toml
[dependencies]
tokio = { version = "*", features = ["full", "signal", "process"] }
serde = { version = "*", features = ["derive"] }
ate = "*"
```

Create a main.rs file

```rust
extern crate tokio;
extern crate serde;

use serde::{Serialize, Deserialize};
use ate::prelude::*;

#[derive(Clone, Serialize, Deserialize)]
struct World
{
    commandment: String
}

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    // The default configuration will store the redo log locally in the temporary folder
    let conf = AteConfig::default();
    let builder = ChainBuilder::new(&conf);

    // We create a chain with a specific key (this is used for the file name it creates)
    let chain = Chain::new(builder, &ChainKey::from("universe")).await?;
    
    // We interact with the data stored in the chain-of-trust using a DIO
    let session = AteSession::default();
    let mut dio = chain.dio(&session).await;
    
    // In this example we store some data in the "World" object
    let key = dio.store(World {
        commandment: "Hello".to_string(),
    })?.key().clone();
    dio.commit().await?;
    
    // Now we retreive the data and print it to console
    println!("{} world!", dio.load::<World>(&key).await?.commandment);

    // All errors in ATE will convert into the AteError
    Ok(())
}
```

## Examples

- [Hello World](lib/examples/hello-world.rs)
- [Server/Client](lib/examples/server-client.rs)
- [Publish/Subscribe](lib/examples/bus-or-queue.rs)
- [Service/API](lib/examples/service-api.rs)
- [Exactly-Once Queue](lib/examples/bus-or-queue.rs)
- [Crypto Currency](lib/examples/coin.rs)
- [Banking Example](lib/examples/bank.rs)

## Projects

- [atedb](atedb/README.md)
- [atefs](atefs/README.md)

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)
