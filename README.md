ATE
===

## Navigation

- [Executive Summary](README.md)
- [User Guide for ATE](doc/guide.md)
- [Technical Design of ATE](doc/design.md)

## Why the name?

The origin of the word "mutate" is the latin word '-ate':  
https://www.dictionary.com/browse/mutate

## Summary

ATE is a distributed immutable data store and built in memory based materialized
view with strong encryption and authentication.

What does that mean?

This library is a way of working with data in modern distributed computing.
* ...data is persisted to a distributed commit log (Kafka).
* ...partitions are divided into topics that shard data into physical domains.
* ...streaming of data to the application occurs on demand during method invocation.
* ...while online data is kept up-to-date using caching invalidation and publish/subscribe.
* ...each topic is a crypto-graph with unique asymmetric keys at differentiating nodes.
* ...the root of the chain-of-trust that validates the crypto-graph is DNS(Sec).
* ...strong authentication and authorized is by design built into the data model.
* ...encryption is highly resistant to quantum attacks and uses fine-grained tenant keys.
* ...all this is integrated into a shared-nothing highly portable executable.

## High Level Design

    .--[Java App]---. .--[Java App]---. .--[Java App]---. .--[Java App]---.
    |               | |               | |               | |               |
    |>Inbuilt Kafka<| |>Inbuilt Kafka<| |>Inbuilt Kafka<| |>Inbuilt Kafka<|
    |.-------------.| |.-------------.| |.-------------.| |.-------------.|
    || Partition 1 || || Replica P1  || || Replica P1  || ||             ||
    ||             || || Partition 2 || || Replica P2  || || Replica P2  ||
    || Replica P3  || ||      ^      || || Partition 3 || || Replica P3  ||
    |*-------------*| |*------|------*| |*-------------*| |*-------------*|
    |               |      subscribe
    |                \________|_______________________________
    |                         |                               |
    |  >Crypto-Graph Materiaized View<                        |
    |  .----------------------------------.      session      |
    |  |             dns                  |   .-----------.   |
    |  |              |                   |   |  -token   |   |
    |  |      dao----dao                  |---|  -claims  |   |
    |  |              \                   |   |  -keys    |   |
    |  |               dao                |   |  -timeout |   |
    |  |                                  |   *-----------*   |
    |  +----------------------------------+----------------+  |
    |  |                 HTTP REST Service                 |  |
    *--|-^--^--^--^--^--^--^--^--^--^--^--^--^--+--^--^--^-|--*
       .                         |         +----|----+     .
       .                       [dto]       | headers |     .
       .                         |         +----|----+     .
    .--|----------------------------------------+----------|--.
    |  |                [HTTP REST Client]                 |  |
    |  |                                                   |  |
    |  |          >realtime client side cache<             |  |
    |  +---------------------------------------------------+  |

## References

Its okay to store data in Kafka...  
https://www.confluent.io/blog/okay-store-data-apache-kafka/

Quantum resistant crypto...  
https://en.wikipedia.org/wiki/NTRU

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)
