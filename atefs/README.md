ATE File System
===============

## What is ATE File System?

ATE File System is a distributed file system built on top of the ATE distributed
datastore. While the author is fully aware that the ecosystem of file-systems is
rather saturated this is a good problem to have in the grand scheme of things, this
particular varient has some unique qualities that differentiate it from others.

## What is ATE

[See here](https://github.com/john-sharratt/ate/blob/master/README.md)

## Summary

ATE File System uses FUSE to redirect file system commands to a user-space program that
responds to IO. This user-space program queries an in-memory distributed database that
is synchronized with many consumers and producers. Data is replicated, encrypted, signed
and multi-user safe.

Another key property of the ATE File System is that it is totally software defined which
allows materialized views of structured non-file based data model to also be represented
within the emulated sections of the file system.

Features:

- Very highly scalable (relative to other file systems)
- Low latency reads through local redo log replication
- Write through caching with distributed commits
- Distributed locking on files
- Fully encrypted files and metadata
- Quantum resistant encryption throughout
- Programmable API for emulated files

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
       |                      atefs                        |
       +---------------------------------------------------+
       |                 fuse (/dev/fuse)                  |
       +---------------------------------------------------+
       |                   Linux Kernel                    |
       +---------------------------------------------------+
       |           Linux System Calls (e.g. read)          |
       +---------------------------------------------------+


## QuickStart

Run a server somewhere

```sh
atedb solo
```

Mount the distributed file-system

```sh
sudo apt install fuse3
atefs mount /mnt/test ~/ate/fs tcp://localhost/myfs
```

## Contribution

If you would like to help setup a community to continue to develop this project
then please contact me at [johnathan.sharratt@gmail.com](johnathan.sharratt@gmail.com)