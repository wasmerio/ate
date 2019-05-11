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
   1. [Stateful Mode](#stateful-mode)
   2. [Stateless Mode](#stateless-mode)
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


                           .-------------.
                          _|  Processor  |
                      ___/ |   |     |   |
                __(TCP)    | Memory Disk |
        .------/------.    '---/---------'
        |  Processor  |       /
        |   |     |   |      /
        | Memory Disk |   (TCP)                .-------------.
        '-------\-----'    /                   |  Processor  |
               (TCP)      /                  __|   |     |   |
                .-\------/----.         ____/  | Memory Disk |
                |  Processor  |    __(TCP)     '-------------'
                |   |     |   |___/
                | Memory Disk |
                '-------------'

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
                                    |                 ^--shared stuff  
                           Interconnected Network           
            .----------IP-----------|----------IP-----------.  
        .---+---.               .---+---.               .---+---.  
        |  CPU  |               |  CPU  |               |  CPU  |  
        +-------+               +-------+               +-------+  
        |  MEM  |               |  MEM  |               |  MEM  |  
        +-------+               +-------+               +-------+  
        |   |   |               |   |   |               |   |   |  
       === === ===             === === ===             === === ===  
         (Disks)                 (Disks)                 (Disks)  

Given the very few external dependencies this architecture is considered by the
author to be of the "Shared Nothing" type. Specifically (when operating in the
[stateful mode](#stateful-mode) it has no external state machine or database
that it relies on.

Reference: https://en.wikipedia.org/wiki/Shared-nothing_architecture

There are two modes of operation for the ATE framework, one that honours the
"Shared Nothing" architecture (stateful mode) and one that doesn't. You may
use the following guide when choosing which mode to use:

1. If your use-case is constrained to one geo-graphic location (i.e. a country)
   and is not anticipated to need extreme scales that require special configuration
   of the Kafka cluster (e.g. rack awareness, mirror-maker, etc..) then run in the
   "Stateful Mode"
2. Otherwise run in "Stateless Mode"

### Stateful Mode

When operating in this mode the in-process Kafka and ZooKeeper servers are running
and hence the application is storing the distributed log partitions and indexing
data on the local disk. In this mode the application is a true "Shared Nothing" as
the database is built into the application is distributed over the nodes.

This mode of operation has the following benefits and disadvantages:

* (+1) Its considerable easier to setup with often a single JAR executable to manage.
* (+1) Scaling the total system is easier with less components to worry about often
  increasing the capacity is no more than spinning up more nodes. When moving to
  extreme scale with replication all over the world this advantage may not hold. 
* (-1) Additional custom configuration of the Kafka and ZooKeeper cluster (e.g. rack
  awareness, mirror maker, cluster authentication) are either not possible or
  difficult to code into the application.
* (-1) As the storage engine runs in the application making it stateful extra care
  must be taken when bringing nodes online and taking them offline.
  
Note: Stateful mode is actually a blend of both stateful and stateless nodes. The
DNS records used for bootstrapping the startup will determine which nodes need
to operate the Kafka cluster and which are just plain dumb compute nodesm, thus it
is still possible to scale out an API built onto of Stateful ATE without worrying
about also scaling the stateful elements (i.e. the disks)

### Stateless Mode

In stateless mode the Kafka and ZooKeeper clusters are running externally from the
application which means while it is still a [distributed application](#distributed-computing-architecture)
it is no longer "shared nothing". In effect the nodes of this database all share an
externally hosted distributed commit log.

This mode of operation has the following benefits and disadvantages:

* (+1) Splitting up the scaling components makes it easier to understand the
  performance bottlenecks and scaling limits of the various components.
* (+1) When running in this mode it becomes easier to add additional security
  on top of the Kafka cluster to increase the layered defence. E.g. Firewall
  rules, ZooKeeper and Kafka authentication, etc...
* (-1) More complex setup from a deployment perspective.
* (-1) Less performance is certain deployments as the data held within the
  distributed commit log may need to travel more distance before it arrives
  at the in-memory materialized view. This disadvantage will deminish as the
  total system is scaled to extreme scale with replication all over the world.

## Absolute Portability

## Chain Of Trust

## Implicit Authority

## Fine Grained Security

## Quantum Resistance

## Eventually Consistent Caching

## Undertow and Weld

## Native REST Integrated
