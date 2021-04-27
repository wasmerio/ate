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
        tcp://yourserver.com:5000/yourdb
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
- Either create your own authentication server (auth-server) or use tcp://auth.tokera.com/
- Create a user programatically or using 'atefs create-user' and 'atefs create-token'

(See the 'atefs' source-code for examples on how to do all this)
```

## Changelog

```
0.6.0  -= Group access rights =-
       + Authentication server now has full group membership support with various
         roles and permission functionality - see auth-tools for details!
       + AteFS now mirrors the chain-of-trust permissions and encryption keys with
         the 'chmod' linux permissions allowing users to easily protect their data
         but share securely with others
       + Lists files and folders should ignore those that you do not have access to
         rather than throwing an error, this means you can partially access file
         systems you only have moderate access to.
       + File-systems should record the correct uid and gid within ATE but change it
         to the actual user when mounted if it matches. Using the chown command
         should allow the object to be given to other users and other groups.
       
       -= Bug Fixes =-
       + Changing the access rights from a private group to everyone should then
         allow the data to be read by everyone without having to resave all the data.
         Effectively this is automatic key-rotation.
       + When attempting to access a group that is not this group AteFS will now
         automatically gather the permissions it needs from the authentication server.
       + Modified files no longer show zeros at the end of the file without actually
         writing the data itself.
       + Changing the permissions on a parent was not reflected in the children as inherited
         rows that are encrypted did not automatically gain the new keys of the parent. Will
         need to store a read-key in the parent which the children use when they are in
         inheritance mode
       + Fixed a bug where files opened with truncate flag were not actually truncating
       + Fixed a major bug where parents were not inheriting permissions properly when the
         parent tree exceeded 1 levels in the chain-of-trust.
       + Fixed a major bug which was causing events to be sent to all other connections on
         the ATE servers even if they were for other chains!
<=0.5.0 See commit history
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


## Quick Start

Add ate, serde and tokio to your dependency list in Cargo.toml

```toml
[dependencies]
tokio = { version = "*", features = ["full", "signal", "process"] }
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
