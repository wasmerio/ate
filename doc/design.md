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

Data stored within ATE is by design "immutable" following the concept of a "log-based
architecture"

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
In order to provide a service that allows for random access of that data (which
of course is necessary for most useful applications) then the log must be streamed
and transformed into a materialized view - the case of ATE the materialized view is
stored in memory as a DAG ([Directed acyclic graph](https://en.wikipedia.org/wiki/Directed_acyclic_graph)).

For the use-cases that ATE is designed for it, the "immutable data" implementation
it usess must meet four key requirements:

1. All events must be retained forever due to need to validate the integrity of the
   cryptographic chain-of-trust materialized view that they feed.
2. The logs must be produced and consumed in the exact same order as seen by all
   consumers - this is essential for the eventual consistency of the data merging
   and transactions. Further, the crypto validation techniques mandate that all data
   events must be processed in topological order for integrity reasons thus
   row ordering is a critical characteristic of the distributed log implementation. 
3. The distributed commit log must support sharding of data at arbitrary boundaries
   in order to meet data locality and data sovereignty requirement(s). Due to
   the second key requirement stated above these arbitrary boundaries must be clean
   cut breaks in the security domain hence splitting integrity into many separate
   chains rather than a monolithic chain. Sharding is also required to achieve high levels of
   parallelism that makes the system linearly scable and to reduce the memory
   footprint of the in-memory materialized views.
4. The distributed commit log must support multi-producers concurrent with
   multi-consumers to meet the later end of the scalability curve.
   
These ([and other reasons](https://medium.freecodecamp.org/what-makes-apache-kafka-so-fast-a8d4f94ab145))
are why this implementation uses Kafka as its storage backend for the distributed
commit log. Ultimately the characteristics of a Kafka cluster setup with specific
configuration that makes it persistent  creates one of the best implementations
of "immutable data" out there.

Reference: https://www.confluent.io/blog/okay-store-data-apache-kafka/  
Reference: https://www.confluent.io/blog/publishing-apache-kafka-new-york-times/  
Reference: https://en.wikipedia.org/wiki/Directed_acyclic_graph  
Reference: https://medium.freecodecamp.org/what-makes-apache-kafka-so-fast-a8d4f94ab145  

## Distributed Computing Architecture

The core architecture of this framework is that its operating state (run-time)
achieves the characteristics of a [**distributed program**](https://link.springer.com/chapter/10.1007/978-1-84882-745-5_11).

This ideal model would look something like this:

* Each node has its own **local memory** that operates fully independently.  
* Nodes communicate with each other by **message passing**.  
* The system has built-in **node failure tolerance**.  
* That the system as a whole is **linearly scalable**.  
* Every node has a context **limited view** of the overall system.  
* **Network topology is unknown** at design and deploy time.  
* Operates using **peer-to-peer** based network topology.  


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

The ATE framework comes close to meeting these ideal characteristics as (when
running in [stateful mode](#stateful-mode)) it operates with these properties:

1. It can be compiled down to a single JAR binary with embedded shared configuration
   files.
2. It uses the distributed DNS infrastructure of the Internet to bootstrap itself
   during startup and to validate the various roots for each chain-of-trust.
3. It excels on high throughput network connectivity even when those networks
   display moderate packet loss and latency (a.k.a. The Internet).
4. Data is distributed across the local disks wherever the ATE binary is running
   Kafka nodes, while data integrity during partition events is maintained through
   data replication.

_Given the properties above it is appropriate to classify the ATE framework as
a "Distributed Computing Architecture"_ suitable for large scale deployments.

Reference: https://en.wikipedia.org/wiki/Distributed_computing  
Reference: https://en.wikipedia.org/wiki/Single_point_of_failure

## Shared Nothing

In a "Shared Nothing" architecture the idea is that all external dependencies
outside of the nodes are kept to an absolute minimum. Ideally there should be no
external dependencies at all but obviously this is a purist view which in reality is
impossible however ATE does come quite close to achieving this by using architectural
patterns and design constraints to remove and eliminate as many external
dependencies as possible.

ATE has the following (external) dependencies:

1. A network connection between all nodes that supports IP packets.
2. DNS services available to be queried via the DNS protocol. 

                                                       
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

Given the very few mandatory external dependencies required by this architecture it
is considered by the author to be of the "Shared Nothing" type. Specifically, when
operating in the [stateful mode](#stateful-mode) it has no external state machine
or database that it relies on.

Reference: https://en.wikipedia.org/wiki/Shared-nothing_architecture

There are two modes of operation for the ATE framework, one that honours the
"Shared Nothing" architecture ([stateful mode](#stateful-mode)) and one that doesn't
([stateless mode](#stateless-mode). You may use the following guide when choosing
which mode to most appropriate for your use-case:

1. If your use-case is constrained to one geo-graphic location (i.e. a country)
   and is not anticipated to need extreme scale and hence require the associated
   necessary extra setup of the Kafka cluster (e.g. rack awareness, mirror-maker,
   etc..) then run ATE in its "Stateful Mode".
2. Otherwise run in "Stateless Mode".

### Stateful Mode

When operating in this mode Kafka and ZooKeeper servers are running in-process 
and hence the application is storing the distributed log partitions and indexing
data on the local disk where the Java application is running. In this mode the
application is a true "Shared Nothing" as the persistent storage is built into the
application itself.

This mode of operation has the following benefits and disadvantages:

* (+1) Its considerable easier to setup, often only requiring a single JAR
  executable to deploy and scale horizontally.
* (+1) Scaling the total system is easier with less components to worry about often
  increasing the capacity is no more than spinning up more nodes. When moving to
  extreme scale with replication all over the world this advantage may not hold
  as the limitations of [CAP theorem](https://en.wikipedia.org/wiki/CAP_theorem)
  become more apparent requiring custom setups of Kafka to fine-tune the
  trade-offs. 
* (-1) Any additional custom configuration of the Kafka and ZooKeeper cluster (e.g.
  rack awareness, mirror maker, cluster authentication) are either not possible to
  run in this mode of operation or are not yet built into the bootstrapping
  process.
* (-1) As the storage engine runs in the application this making it stateful,
  extra care must be taken when bringing nodes online and taking them offline.
  
Note: Stateful mode is actually a blend of both stateful and stateless nodes. The
DNS records used for bootstrapping the startup will determine which nodes need
to operate the Kafka cluster and which are just plain dumb compute nodes - thus -
it is still possible to scale out an API built onto of Stateful ATE without
worrying about also scaling the stateful elements (i.e. the disks)

### Stateless Mode

In stateless mode the Kafka and ZooKeeper clusters are running externally from the
application which means while it is still a [distributed application](#distributed-computing-architecture)
it is no longer "Shared Nothing" as in effect the nodes are simply compute nodes
while the actual data is instead persistent on an externally hosted distributed
commit log (bespoke Kafka cluster).

This mode of operation has the following benefits and disadvantages:

* (+1) Splitting up the scaling components makes it easier to understand the
  performance bottlenecks and scaling limits of the various components.
* (+1) When running in this mode it becomes easier to add additional security
  on top of the Kafka cluster to increase the layered defence. E.g. Firewall
  rules, ZooKeeper and Kafka authentication, etc...
* (-1) This is a more complex setup from a deployment perspective.
* (-1) Less performance in certain small scale deployments as the data held within
  the distributed commit log may need to travel more distance before it arrives
  at the in-memory materialized view. This disadvantage will diminish as the
  total system is scaled to medium sized deployments.

## Absolute Portability

Better Portability reduces the cost of deploying, operating and testing applications
thus ensuring ATE has the best portability properties was an important factor in
its development.


         ^                 |
        S|                 |       [goal] X
        I|                 |
        M|                 |
        L+-----------------+------------------
        I|                 |
        C|                 |
        I|                 |
        T|                 |
        Y+-----------------+----------------->
                 P O R T A B I L I T Y

A measure of portability is the effort it takes to move the software and/or
application from one environment to another environment. Ideally this should be
possibly at zero cost, ATE gets closer than most to this ideal state.

ATE applies the following design constraints:

* Target intermediate output such as [Bytecode](https://nl.wikipedia.org/wiki/Bytecode)
  in the case of the ATE framework - Java is used to generate the Bytecode.
* Configuration is code and thus portability optimizations applied to code are also
  applicable to the configuration files themselves.
* Configuration files distributed with the applications must be the same regardless
  of which node in the cluster they are deployed to - hand crafted configuration
  files are an anti-pattern.
* DNS of the environment is used to externalize configuration from the
  application configuration files so that applications become "environment aware"
  ultimately this is a realization of the [convention over configuration](https://en.wikipedia.org/wiki/Convention_over_configuration)
  pattern.
* Test environments should simulate the DNS entries by intercepting the queries
  and impersonating the results, this ensures the environment specific settings
  the configuration files are shipped with become immutable-per-release meaning
  they are shifted to the [left of testing](https://en.wikipedia.org/wiki/Shift_left_testing).
* The temptation to define environment awareness in the application should
  be avoided thus eliminating the need for environment conditional logic
  in the code and custom deployment pipelines for each environment.

Which means the only thing required to be changed between environments is the
following:

* DNS entries in the environment that the application is running where the
  domain names are dependent on the application configuration for its use case
  regardless of the environment it is deployed to.  
  DNS entries required for this solution are as follows:
  1. DNS Entries that determine where ZooKeeper runs in the environment.
  2. DNS Entries that determine where Kafka runs in the environment.
  3. DNS Entries that hold the root public keys for the chain-of-trust seeding.
* Configuration file settings for the application are the same regardless of
  which environment they are configured to except for the environment specific
  authentication credentials that segregate security domains. These particular
  settings should be kept to an absolute minimum where possible.
  
Reference: https://en.wikipedia.org/wiki/Software_portabilityhttps://en.wikipedia.org/wiki/Software_portability  
Reference: https://nl.wikipedia.org/wiki/Bytecode  
Reference: https://en.wikipedia.org/wiki/Convention_over_configuration  
Reference: https://en.wikipedia.org/wiki/Shift_left_testing  

## Chain Of Trust

Ultimately the need for authentication, authorization, encryption and integrity
checks can be (somewhat overly) generalized as the following two keys goals:

1. Preventing people from reading information that they should not know.
2. Preventing people from writing information in areas they are not allowed.

Where systems often fail in this challenge is attacks are able to take advantage
in weaknesses in the design somewhere between where the information resides and
legitimate humans that the system is designed for.

     
                            Chain Of Trust                                              I I I
                            ==============->                                            N N N 
     H                                                                                  F F F
     U   .----------|-----------------|----------------|---------------|------------.   O O O
     M   | Identity | Trust Authority | Authentication | Authorization | Encryption |   R R R
     A   '--^-------|----------------^|----------------|----^----------|--------^---'   M M M
     N      |                        |                      |                   |       A A A
          attack!                    |                    attack!             attack!   T T T
                                   attack!                                              I I I
                                                                                        O O O
                                                                                        N N N

## Implicit Authority

## Fine Grained Security

## Quantum Resistance

## Eventually Consistent Caching

## Undertow and Weld

## Native REST Integrated
