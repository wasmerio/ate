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

- [Typical Client](auth/examples/client.rs)
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
     '------|----\-'          '- - - - - - -'
            |     \                 
        ws://yourserver.com/db
            |       \
     .------|------. \
     |Native Client|  .-----Browser-----.
     |             |  |.---------------.|
     | >program    |  || >wasm32-wasi  ||
     |  \ate.so    |  ||  \ate.wasm    ||
     '-------------'  |'---------------'|
                      '-----------------'

The easiest way to get up and running is to just build your app and point the
database URL at ws://tokera.sh/db. You will need to register an account and verify
your identity however after this you can use the free databases and/or paid option.

Alternatively, if you wish to host your own ATE servers in infrastructure that you
manage and run then follow these high-level steps.

1. Server runs the 'atedb' process on some network reachable location
2. Create several records for each IP address under the same A-record in your DNS
3. Either create your own authentication server as well using the auth-server binary
   or just use the authentication servers hosted at Tokera by pointing to
   ws://tokera.sh/auth.
```

## Quick Start

Cargo.toml

```toml
[dependencies]
tokio = { version = "*", features = ["full", "signal", "process"] }
serde = { version = "*", features = ["derive"] }
ate = { version = "*" }
ate_auth = { version = "*" }
```

main.rs

```rust
use serde::{Serialize, Deserialize};
use ate_auth::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MyData
{
    pi: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    let dio = DioBuilder::default()
        .with_session_prompt().await?
        .build("mychain")
        .await?;

    dio.store(MyData {
        pi: "3.14159265359".to_string(),
    })?;
    dio.commit().await?;

    Ok(())
}
```

## Changelog

```
1.2.1  -= Lazy Loading =-
        + Subscribing to chains can now load the data in the chain on demand as its needed
          which reduces the startup time considerably.
        + Temporal clients will default to lazy loading
        + Data loaded via the lazy loading mechanism will now be cached client side

1.1.1  -= Performance and Bug Fixes =-
        + Fixed an issue with the web sockets that caused sporadic disconnects
        + Improved the performance of web socket messages by reusing IV's
        + Reduced the message overhead with a new message encoding format

1.1.0  -= Comms Upgrade =-
        + Streaming websockets are now more stable as they use length headers to delimit messages.
        + Fixed a bug where disconnecting clients would drop neighbors on the same server.
        + Various changes to the interfaces for stability reasons
        (this upgrade is not backwards compatible with version 1.0.6)

1.0.6  -= Bug Fixes =-
        + Modified the interface slightly but most users should not be impacted
        + Fixed a bug around validators rejecting events during the subscribe
          process that re-reads them from disk - these validators should not be running
        + Added the ability to list all root objects
        + Added the ability to delete all root objects (and hence wipe a chain)
        + Fixed a serious deadlock situation when commiting transactions that was causing timeouts

1.0.2  -= WASM BUS =-
       + Integrated with the WASM bus (wasm-bus) which allows for ATE to use
         the web sockets while running in a controlled sandbox.

1.0.0  -= Major Release =-
       + See [README.md](https://github.com/tokera-com/ate/blob/e0beedbbbd84f95cd6c7a9a45b8903058f65b6fd/README.md)

<=0.8.0 See commit history
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
    |  |             root(hash)           |   .-----------.
    |  |              |                   |   |  -token   |
    |  |      dao----dao(aes)             |---|  -claims  |
    |  |              \                   |   |  -keys    |
    |  |               dao                |   *-----------*
    |  |                                  |


## Feature Flags

- 'client'     - Client functionality that allows one to connect to ATE datachains
                 and/or host them locally
- 'server'     - Server functionality required to create and run ATE in distributed
                 mode with the data replicated on server nodes.
- 'client_web' - Client functionality designed for running within a browser sandbox
                 (--target=wasm32-wasi)

## WebAssembly

When compiling for WASM use the following command:

```sh
cargo build --target wasm32-wasi --no-default-features --features client_web
```

## Lower Level Example

Cargo.toml

```toml
[dependencies]
tokio = { version = "*", features = ["full", "signal", "process"] }
serde = { version = "*", features = ["derive"] }
ate = { version = "*" }
```

main.rs

```rust
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
