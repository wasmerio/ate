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
* ...streaming of data to the application occurs on demand as the app needs it.
* ...each chain is a crypto-graph with unique asymmetric keys at differentiating nodes.
* ...the root of the chain-of-trust validates the crypto-graph through various plugins.
* ...strong authentication and authorized is by design built into the data model.
* ...encryption is highly resistant to quantum attacks and uses fine-grained tenant keys.
* ...all this is integrated into a shared-nothing highly portable executable.

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

## Typical Deployment Pattern

```
     .-------------.          .- - - - - - -.
     |   Server    |              Server
     |             | .. .. .. |             | .. .. ..
     | >atedb solo |
     '------|------'          '- - | - - - -'
            |                      |
        ws://yourserver.com/db/yourdb
            |
     .------|------.
     |   Client    |
     |             |                 
     | >program    |
     |  \ate.so    |
     '-------------'

- Server runs the 'atedb' process on some network reachable
- Create several records for each IP address under the same A-record in your DNS
- Build your model in RUST and add ATE as a dependency
- Either create your own authentication server (auth-server) or use ws://tokera.com/auth
- Create a user programatically or using 'atefs create-user' and 'atefs create-token'

(See the 'atefs' source-code for examples on how to do all this)
```

## Changelog

```
0.8.0  -= Stability Release =-

       + Fixed an infinite loop and locked-up bug for certain scenarios that have
         a tree structure of data objects
       + Added more helper functions that make the library more extendable
       + Removed the log file ".redo" prefix, this may require you to rename your
         log files after upgrading
       + Added default ports on all the URL's to reduce boiletplate and help
         prevent easily avoidable configuration errors
       + Some minor changes in the structure of the library to help with extendability
       + Improved DaoRef object that works across chains
       + Added additional error handling for certain failure modes
       + Fixed a bug where commands were not aborting when the connection fails with the server
       + Refactoring of the data model behind ATE's key authentication objects
       + Added a regex to check the email address is valid
       + The wire-format of connections will now properly negotiate even if they are
         configured differently between server and client.
       + Fixed a bug where compacting events would sometimes lose data in scenarios where
         connectivity was lost during the compact processing.
       + Fixed a bug where worker threads were not properly exiting which prevented a
         graceful shutdown of the process.

0.7.0  -= Compacting Chains =-

       + ATE now fully supports compacting chains - compacting chains works both
         client side and server side completely independently which allows either
         the server or client to compress down redo-logs by removing all duplicate
         events and anything that has been tombstoned (a bit like Kafka compacting
         topics does but with more intelligence in the process - e.g. validators)
       + Made the local file storage of redo logs optional rather than mandatory
         thus users of ATE can now run entirely in memory (this is in preparation
         for refactoring ATE for use in WebAssembly)
       + Chain history now uses the NTP clock for building its timelines thus
         with a bit of tolerance added into the mix it becomes possible to make
         a more stable sync process between multiple clients and servers
       
       -= Bug Fixes =-

       + Fixed a bug where connections were left open even when the chains went out
         of scope (e.g. short commands) - this was leaking thus hurted stability
       + Fixed a major bug where the redo-logs would become corrupted in specific
         scenarios where the log is reloaded but the offsets were not updated.
       + Fixed a security flaw in the events streamed from the servers which
         included the computed hash for the data however this needs to be recomputed
         client-side in case someone tries to spoof the event.
       + Fixed various other minor bugs

<=0.6.0 See commit history
```

## High Level Design

    .--[ atedb  ]---. .--[ atedb  ]---.      .-[auth-server]-.
    |               | |               |      |               |
    |>local redo-log| |>local redo-log|      |>local redo-log|
    |.-------------.| |.-------------.|      |.-------------.|
    || Chain     1 || ||             ||      ||    user     ||
    ||             || || Chain     2 ||      ||   account   ||
    |*-------------*| |*------|------*|      |*-----|-------*|
    |               |       subscribe             login      
    |                \________|_____________________|____
    |                         |                     |    
    |  >local redo-log                                   
    |  >Crypto-Graph Materiaized View< (in memory)       
    |  .----------------------------------.      session 
    |  |             root                 |   .-----------.
    |  |              |                   |   |  -token   |
    |  |      dao----dao                  |---|  -claims  |
    |  |              \                   |   |  -keys    |
    |  |               dao                |   |  -timeout |
    |  |                                  |   *-----------*


## Feature Flags

- 'client'     - Client functionality that allows one to connect to ATE databases
                 and/or host them locally
- 'server'     - Server functionality required to create and run ATE in distributed
                 mode with the data replicated on server nodes.
- 'client_web' - Client functionality designed for running within a browser sandbox
                 (--target=wasm32-wasi)

## Quick Start

Add ate, serde and tokio to your dependency list in Cargo.toml

```toml
[dependencies]
tokio = { version = "*", features = ["client", "server"] }
serde = { version = "*", features = ["derive"] }
ate = { version = "*", features = ["all"] }
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
    let builder = ChainBuilder::new(&conf).await.build();

    // We create a chain with a specific key (this is used for the file name it creates)
    let chain = builder.open(&ChainKey::from("universe")).await?;
    
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

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)
