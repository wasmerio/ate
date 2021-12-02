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
database URL at ws://tokera.com/db. You will need to register an account and verify
your identity however after this you can use the free databases and/or paid option.

Alternatively, if you wish to host your own ATE servers in infrastructure that you
manage and run then follow these high-level steps.

1. Server runs the 'atedb' process on some network reachable location
2. Create several records for each IP address under the same A-record in your DNS
3. Either create your own authentication server as well using the auth-server binary
   or just use the authentication servers hosted at Tokera by pointing to
   ws://tokera.com/auth.
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
1.0.2  -= WASM BUS =-
       + Integrated with the WASM bus (wasm-bus) which allows for ATE to use
         the web sockets while running in a controlled sandbox.

1.0.0  -= Major Release =-

       + Major compatibility breaking refactor of the code to bring it up to
         production grade. All future changes will be fully forwards compatible
         for all versions within this major.
       + Added support for WASM with wasm32-wasi as a build target. This will
         allow ATE to be compiled and used directly within WebAssembly. More on
         this new feature coming soon!
       + Added full support for multiplexed WebSockets (ws://) which allows ATE
         to be consumed via HTTP calls including directly from the browser.
       + Added 'path' awareness to the hello messages so that multiple ATE servers
         can listen on the same port.
       + Implemented full support for async single threaded clients and servers
         which will allow ATE to run at high performance in a browser sandbox.
       + Commands will now reuse connects thus lowering the connection negotiation
         frequence and improving performance.
       + Registries will now keep chains alive for a fixed period of time (60s)
         after closed to improve responsiveness and performance when reused.
       + Added the ability to throttle or rate-limit connections.
       + Major improvements to the DAO and DIO access objects that make them much
         easier to use and provide a full feature set.
       + Implemented DaoMap that allows for fast access to hashed map objects.
       + Improved the serialization code to produce smaller log files and improve
         the performance during loading of chains.
       + Switched from SHA3 to Blade3 as the default hasher to improve performance
         but retain strong cryptographic security.
       + Added Client and Server certificates so that chains running in centralized
         trust mode are resistant to man-in-the-middle denial of service attacks.
       + Implemented an automatic backup and restore capability.
       + Implemented a redo file dump helper utility useful for debugging purposes.
       + Added a user account recovery process for when users lose their password.
         This recovery process does not compromising on the core values of ATE.
       + All user accounts created now include an email verification step for
         proof of ownership.
       + Creating (domain) groups they are now validated against TXT DNS records
         for proof of ownership.
       + Linked in Tokera functionality that allows for Wallets and Contracts to
         be used that facilitates a secure way to pay for Tokera services.
       + Implemented free and paid hosting of ATE databases on http://tokera.com
       + Switched to a new error handling framework (crate error_chain) to reduce
         code bloat and make better errors.
       + Switched to a new logging framework (crate:tracing) which allows for much
         better debugging and operations support.
       + Chain compacting no longer breaks the trust chains on centralized trees
       + Authentication server is now using compacting chains.
       + Many significant performance enhancements.
       + Fixed quite a number of major and minor bugs and improved the overal
         stability to the point that this is now a major release.

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
