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

    .--[   App  ]---. .--[   App  ]---. .--[   App  ]---. .--[   App  ]---.
    |               | |               | |               | |               |
    |>local redo-log| |>local redo-log| |>local redo-log| |>local redo-log|
    |.-------------.| |.-------------.| |.-------------.| |.-------------.|
    || Chain     1 || || Replica P1  || || Replica P1  || ||             ||
    ||             || || Chain     2 || || Replica P2  || || Replica P2  ||
    || Replica P3  || ||      ^      || || Chain     3 || || Replica P3  ||
    |*-------------*| |*------|------*| |*-------------*| |*------^------*|
    |               |       subscribe                             |
    |                \________|_______________________________  subscribe
    |                         |                               |   .
    |  >local redo-log                                        |   .
    |  >Crypto-Graph Materiaized View< (in memory)            |   .
    |  .----------------------------------.      session      |   .
    |  |             root                 |   .-----------.   |   .
    |  |              |                   |   |  -token   |   |   .
    |  |      dao----dao                  |---|  -claims  |   |   |
    |  |              \                   |   |  -keys    |   |   .
    |  |               dao                |   |  -timeout |   |   .
    |  |                                  |   *-----------*   |   |
    |  +----------------------------------+----------------+  |   .
    |  |                [HTTP REST Service]                |  |   .
    *--|-^--^--^--^--^--^--^--^--^--^--^--^--^--+--^--^--^-|--*   |
       .                         |         +----|----+     .      .
       .                       [dto]       | headers |     .      |
       .                         |         +----|----+     .      .
    .--|----------------------------------------+----------|--.   |
    |  |                [HTTP REST Client]                 |  |   |
    |  |                                                   |  |   .
    |  |        >realtime client side cache<--(invalidate)_|__|___/
    |  +---------------------------------------------------+  |

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)
