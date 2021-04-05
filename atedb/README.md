ATE Database
===============

## What is ATE Database?

ATE Database is a distributed redo log server that the ATE projects can use to
remotely store their redo logs.

## What is ATE

[See here](../README.md)

## Summary

ATE Database runs a server daemon that listens for connections from clients and serves
as a distributed redo log.

Other projects use this backend for persistent storage - projects such as

- [tokfs](../tokfs/README.md)

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
       +----------------------------------+----------------+
       |                      atedb                        |
       +---------------------------------------------------+
       |                   /opt/ate/*                      |
       +---------------------------------------------------+


## QuickStart

```sh
atedb solo
```

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)