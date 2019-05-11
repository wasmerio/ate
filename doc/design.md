ATE Technical Design
====================

## Navigation

- [Executive Summary](../README.md)
- [User Guide for ATE](guide.md)
- [Technical Design of ATE](design.md)
- [Component Design of ATE](components.md)

## Table of Contents

1. [Immutable Data](#immutable-data)
2. [Distributed Computing Architecture](#distributed-computing-architecture)
3. [Shared Nothing](#shared-nothing)
4. [Absolute Portability](#absolute-portability)
5. [Chain of Trust](#chain-of-trust)
6. [Implicit Authority](#implicit-authority)
7. [Fine Grained Security](#fine-grained-security)
9. [Quantum Resistent](#quantum-resistent)
10. [Eventually Consistent Caching](#eventually-consistent-caching)
11. [Native REST Integrated](#native-rest-integrated)

## Immutable Data

Data stored within ATE is by design "immutable" following the "log-based architecture":

                                                                  Next
                                                                 Record
    1st Record                                                   Written
        |                                                           |
        V                                                           V
      +---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
      |   |   |   |   |   |   |   |   |   |   |   |   |   |   |   |   |
      | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10| 11| 12| 13| 14| 15|
      |   | | |   |   |   |   |   |   |   |   |   |   |   |   |   |   |
      +---+-|-+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
            |
           DAO 

Once a record is written to the distributed commit log it remains there forever.
In order to provide a service that allows for random access of the data (which
is necessary for APIs for instance) then the log must be streamed into a
materialized view, the case of ATE the materialized view is stored in memory.

Four key requirements are needed for the immutable data (and hence the immutable
log) for the use-cases that ATE is designed to that supports:

1. All events must be retained forever due to the cryptographic chain-of-trust.
2. The logs must be produced and consumed in the exact same order as seen by all
   consumers - this is essential for the eventual consistency of the data merging
   and transactions. Further, the crypto validation mandates that all data
   elements must be processed in topological order for integrity reasons thus
   row ordering is critical characteristic of the distributed log. 
3. The distributed commit log must support sharding of data at arbitrary boundaries
   in order to meet data locality and data sovereignty requirement. Due to
   requirement (2.) stated above these arbitrary boundaries must be clean cuts in
   in the security domain thus splitting integrity into many separate chains rather
   than a monolithic chain. Sharding is also required to achieve high levels of
   parallelism that makes the system linearly scable and to reduce the memory
   footprint of the in-memory materialized views.
4. The distributed commit log must support both multi-producers and multi-consumers
   active at the same time to meet the need for extreme scalability.
   
These (and other reasons) are why this implementation uses Kafka as its storage
backend for the distributed commit log. Ultimately the characteristics of a Kafka
cluster setup with specific configuration is what makes "immutable data" possible. 

Reference: https://www.confluent.io/blog/okay-store-data-apache-kafka/  
Reference: https://www.confluent.io/blog/publishing-apache-kafka-new-york-times/

## Distributed Computing Architecture

The core architecture of this framework is that its operating state (run-time)
achieves the characteristics of a [**distributed program**](https://link.springer.com/chapter/10.1007/978-1-84882-745-5_11).

Under this ideal model:

* Each node has its own **local memory** that operates fully independently.  
* Nodes communicate with each other by **message passing**.  
* The system has built-in **node failure tolerate**.  
* That the system as a whole is **linearly scalable**.  
* Every node has a context **limited view** of the overall system.  
* **Network topology is unknown** at design or deploy time.  
* Operates using **peer-to-peer** connectivity with majority based quorum.  


                           +-------------+
                          _|  Processor  |
                      ___/ |   |     |   |
                __(TCP)    | Memory Disk |
        +------/------+    +---/---------+
        |  Processor  |       /
        |   |     |   |      /
        | Memory Disk |   (TCP)                +-------------+
        +-------\-----+    /                   |  Processor  |
               (TCP)      /                  __|   |     |   |
                +-\------/----+         ____/  | Memory Disk |
                |  Processor  |    __(TCP)     +-------------+
                |   |     |   |___/
                | Memory Disk |
                +-------------+

The ATE framework comes close to meeting these ideal properties as (when built in
the fully stateless configuration) it displays these properties:

1. It can be compiled down to one JAR binary with embedded shared configuration files.
2. It uses the distributed DNS infrastructure of the Internet to bootstrap itself
   during startup and to validate the chains-of-trust.
3. It needs a high throughput network connectivity however this network can be
   both un-guaranteed (packets can be lost) and of moderate latency (global).
   (a.k.a. The Internet).
4. Data is distributed across the local disks wherever the binary is running
   which is replicated multiple times for redundancy and availability reasons.

_Given this the ATE framework can be classified as a truly "Distributed
Computing Architecture"_

Reference: https://en.wikipedia.org/wiki/Distributed_computing  
Reference: https://en.wikipedia.org/wiki/Single_point_of_failure

## Shared Nothing

In a "Shared Nothing" architecture the idea is that all external dependencies
outside of the nodes are kept to an absolute minimum, ideally there should be no
external dependency at all. Obviously this is a pure view on which reality makes
impossible however ATE comes quite close to achieving this by using architectural
patterns and design constraints to remove and eliminate as many external
dependencies as possible..

ATE has the following (external) dependencies:

1. A network connection between all nodes that supports IP packets.
2. DNS services available to be queries via the DNS protocol. 

                                                       
                                 DNS
                                  |
                         Interconnected Network         ^=----shared stuff
          +-----------------------+-----------------------+    
          |                       |                       |      
      +---+---+               +---+---+               +---+---+
      |  CPU  |               |  CPU  |               |  CPU  |
      +-------+               +-------+               +-------+
      |  MEM  |               |  MEM  |               |  MEM  |
      +-------+               +-------+               +-------+
      |   |   |               |   |   |               |   |   |
     === === ===             === === ===             === === ===
       (Disks)                 (Disks)                 (Disks)

Given the very few external dependencies this architecture is considered by the
author to be of the "Shared Nothing" type. Specifically (when operating in the
[stateless mode](#stateless-mode) it has no external state machine or database
 that it relies on.

Reference: https://en.wikipedia.org/wiki/Shared-nothing_architecture

### Stateless Mode

### Stateful Mode

## Absolute Portability

## Chain Of Trust

## Implicit Authority

## Fine Grained Security

## Quantum Resistance

## Eventually Consistent Caching

## Undertow and Weld

## Native REST Integrated
