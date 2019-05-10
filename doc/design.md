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
3. [Share Nothing](#share-nothing)
4. [Absolute Portability](#absolute-portability)
5. [Chain of Trust](#chain-of-trust)
6. [Implicit Authority](#implicit-authority)
7. [Fine Grained Security](#fine-grained-security)
9. [Quantum Resistent](#quantum-resistent)
10. [Eventually Consistent Caching](#eventually-consistent-caching)
11. [Native REST Integrated](#native-rest-integrated)

## Immutable Data

Data stored within ATE is immutable by design following a "log-based architecture":

                                                                  Next
                                                                 Record
    1st Record                                                   Written
        |                                                           |
        V                                                           V
      +---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
      |   |   |   |   |   |   |   |   |   |   |   |   |   |   |   |   |
      | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10| 11| 12| 13| 14| 15|
      |   |   |   |   |   |   |   |   |   |   |   |   |   |   |   |   |
      +---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+

Once a record is written to the distributed commit log it remains there forever,
thus in order to provide a service that allows for random access the log must be
streamed into a materialized view in memory.

Four key requirements are required for the immutable log that supports this
implementation of immutable data:

1. All events must be retained for forever due to the cryptographic chain-of-trust.
2. The logs must be produced and consumed in the exact same order as seen by all
   consumers - this is essential for the eventually consistency of the data merging
   and transaction consistency. Further this requirement is needs as the data
   elements must be processed in topological order for integrity reasons.
3. The distributed commit log must support sharding of data at arbitrary boundaries
   in order to meet data locality and data sovereignty requirement. Further
   is required to achieve high levels of parallelism and smaller memory footprints
   on the materialized views.
4. The distributed commit log must support both multi-producers and multi-consumers
   active at the same time to meet the need for extreme scalability.
   
These (and other reasons) are why this implementation uses Kafka as its storage
backend for the distributed commit log and thus ultimately to provide the
capability of "immutable data". 

Reference: https://www.confluent.io/blog/okay-store-data-apache-kafka/  
Reference: https://www.confluent.io/blog/publishing-apache-kafka-new-york-times/

## Distributed Computing Architecture

The core architecture of this framework is that its operating state (run-time) is
follow characteristics of a **distributed program**.

Under this ideal model:

- Each node has its own **local memory** that operates fully independently.
- Nodes communicate with each other by **message passing**
- The system has built-in **node failure tolerate**.
- That the system as a whole is **linearly scalable**.
- Every node has a context **limited view** of the overall system.
- **Network topology is unknown** at design or deploy time.
- Operates using **peer-to-peer** connectivity with majority based quorum.


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

This framework comes close to meeting these ideal properties with the following
operational requirements (when built in the recommended way):

1. It can be compiled down to one JAR binary with embedded shared configuration files.
2. It uses the distributed DNS infrastructure of the Internet to bootstrap itself
   during startup and to validate the chains-of-trust thus re-using highly
   robust and proven central trust authorities that are accessible to everyone.
3. It needs a high throughput network connectivity however this network can be
   both un-guaranteed (packets can be lost) and of moderate latency (global).
   a.k.a. The Internet.
4. Data is distributed across the local disks wherever the binary is running
   which is replicated multiple times for redundancy and availability reasons.

_Given this the ATE framework can be classified as a truly "Distributed
Computing Architecture"_

Reference: https://en.wikipedia.org/wiki/Distributed_computing  
Reference: https://en.wikipedia.org/wiki/Single_point_of_failure

## Share Nothing

In a "Shared Nothing" architecture dependencies outside of the nodes are kept
to an absolute minimum, ideally there should be no dependency at all. Obviously
this is a pure view on reality however ATE comes quite close to achieving this.

In ATE it has the following (external dependencies):

1. A network connection between all nodes that supports IP packets.
2. DNS services available to be queries via the DNS protocol. 

                                                       
                                DNS
                                 |
                        Interconnected Network         ^=------shared stuff
         +-----------------------+-----------------------+    
         |                       |                       |      
     +---+---+               +---+---+               +---+---+
     |  CPU  |               |  CPU  |               |  CPU  |
     +-------+               +-------+               +-------+
     |  MEM  |               |  MEM  |               |  MEM  |
     +-------+               +-------+               +-------+
     |   |   |               |   |   |               |   |   |
    _I_ _I_ _I_             _I_ _I_ _I_             _I_ _I_ _I_
    === === ===             === === ===             === === ===
      (Disks)                 (Disks)                 (Disks)

Given the very few external dependencies this architecture and the distinct
lack of the need for any database or state server this architecture is considered
by the author to be a "Shared Nothing" type.

Reference: https://en.wikipedia.org/wiki/Shared-nothing_architecture

## Absolute Portability

## Chain Of Trust

## Implicit Authority

## Fine Grained Security

## Quantum Resistance

## Eventually Consistent Caching

## Undertow and Weld

## Native REST Integrated
