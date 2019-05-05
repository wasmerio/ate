ATE
===

## Navigation

- [Executive Summary](README.md)
- [User Guide for ATE](doc/guide.md)
- [Technical Design of ATE](doc/design.md)

## Summary

ATE is a distributed immutable data store and built in memory based materialized
view with strong encryption and authentication.

What does that mean?

This library is a way of working with data in modern computing.
* ...data is persisted to a distributed commit log (Kafka).
* ...partitions are divided into topics that represent each account.
* ...streaming of data to the application occurs on demand during login.
* ...while online data is kept up-to-date using caching invalidation.
* ...each account is a crypto-graph with unique asymmetric keys.
* ...strong authentication and authorized is design built-in to the data model.
* ...encryption is highly resistant to quantum attacks and highly segregated.

## References

Its okay to store data in Kafka  
https://www.confluent.io/blog/okay-store-data-apache-kafka/

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
    |  |           account                |   .-----------.   |
    |  |              |                   |   |  -token   |   |
    |  |   object--object                 |---|  -claims  |   |
    |  |              \                   |   |  -keys    |   |
    |  |               object             |   |  -timeout |   |
    |  |                                      *-----------*   |
    |                                                         |
    |  +---------------------------------------------------+  |
    |  |                 HTTP REST Service                 |  |
    *--|-^--^--^--^--^--^--^--^--^--^--^--^--^--^--^--^--^-|--*

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)
