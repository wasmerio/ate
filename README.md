ATE
===

## Navigation

- [Executive Summary](README.md)
- [User Guide for ATE](doc/guide.md)
- [Technical Design of ATE](doc/design.md)
- [Component Design of ATE](doc/components.md)

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
    |                \________|_______________________________ 
    |                         |                               |
    |  >local redo-log                                        |
    |  >Crypto-Graph Materiaized View< (in memory)            |
    |  .----------------------------------.      session      |
    |  |             root                 |   .-----------.   |
    |  |              |                   |   |  -token   |   |
    |  |      dao----dao                  |---|  -claims  |   |
    |  |              \                   |   |  -keys    |   |
    |  |               dao                |   |  -timeout |   |
    |  |                                  |   *-----------*   |
    |  +----------------------------------+----------------+  |
    |  |                                                   |  |

## Quick Start

Add ate, serde and tokio to your dependency list in Cargo.toml

    '''toml
    [dependencies]
    tokio = { version = "1.3", features = ["full", "signal", "process"] }
    serde = { version = "1.0", features = ["derive"] }
    ate = "*"
    '''

Create a main.rs file

    '''rust
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
        
        // Now we retreive the data and print it to console
        println!("{} world!", dio.load::<World>(&key).await?.commandment);
    
        // All errors in ATE will convert into the AteError
        Ok(())
    }
    '''

## Examples

- [Hello World](examples/hello-world.rs)
- [Server/Client](examples/server-client.rs)
- [Fast Event BUS](examples/bus.rs)
- [Crypto Currency](examples/coin.rs)
- [Banking Example](examples/bank.rs)

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)
