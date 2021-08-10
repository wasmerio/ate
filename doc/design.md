ATE Technical Design
====================

## Navigation

- [Executive Summary](../README.md)
- [User Guide for ATE](guide.md)
- [Technical Design of ATE](design.md)
- [Component Design of ATE](components.md)

## Table of Contents

1. [Immutable Data](#immutable-data)
2. [Eventually Consistent Caching](#eventually-consistent-caching)
3. [Caching API Responses](#caching-api-responses)
4. [Materialized Views and Secondary Indexes](#materialized-views-and-secondary-indexes)
5. [Distributed Computing Architecture](#distributed-computing-architecture)
6. [Shared Nothing](#shared-nothing)
   1. [Stateful Mode](#stateful-mode)
   2. [Stateless Mode](#stateless-mode)
7. [Absolute Portability](#absolute-portability)
8. [Chain of Trust](#chain-of-trust)
9. [Implicit Authority](#implicit-authority)
10. [Fine Grained Security](#fine-grained-security)
11. [Quantum Resistent](#quantum-resistent)
12. [Native REST Integrated](#native-rest-integrated)

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
and transformed into a materialized view - in the case of ATE the materialized view is
stored in memory as a DAG ([Directed acyclic graph](https://en.wikipedia.org/wiki/Directed_acyclic_graph)).

For the use-cases that ATE is designed for it, the "immutable data" implementation
it uses must meet four key requirements:

1. All trust verifiedevents must be retained forever due to need to validate the
   integrity of the cryptographic chain-of-trust materialized view that they feed
   however events can still be tombstoned and compacted.
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

While the original version of ATE used Kafka as its backing store there were a
number of advantages in vertically integrating the redo-log and ATE functional logic
into a common stack. For this reason the redo-log has been built from scratch in
rust.

Reference: https://en.wikipedia.org/wiki/Directed_acyclic_graph  

## Eventually Consistent Caching

ATE is really just a traditional database split into two parts rather than one.

The traditional commit log attached to databases like SQL Server and Oracle has
been split off into a highly scalable distributed commit log. While the
tree data structures and tables of the traditional database have moved far away from
the logs and instead embedded as materialized views within the applications that
actually need the data. i.e. as close to the business logic as possible.


       .-------------------------------.
       | Traditional Database (MS-SQL) |              .-------------------.
       +-------------------------------+              | Materialized View |
       | .-------------------.         |              +-------------------+
       | | Relational Engine |<--.     |              |   Buffer Cache    |
       | '-------------------'   |     |              '----------^--------'
       |                     .---|--.  |                         |
       |  Transaction Log    |      |  |                         | (async)    
       |  (-----------() <-->|      |  |  [split]>    /\/\/\/\/\/|/\/\/\/\/\
       |    ____             |      |  |                         |
       |  .=    =.           |      |  |             Distributed | Commit Log
       |  |'----'|<--------->|      |  |                (--------|------()
       |  |      |           |      |  |                (--------|------()
       |   ------            '------'  |                (--------|------()
       |  Data File           Buffer   |                (--------+------()
       '-------------------------------'                (---------------()

This presents challenges, opportunities and solutions.

Challenges:

* How to keep the Materialized Views in sync across multiple applications.
* How to maintain transaction consistency across applications.
* The materialized view will use lots of memory if not careful designed.

Opportunities:

* If the materialized views can be kept in sync then they can also serve as a highly
  efficient client-side cache.
* Keeping data very close to the application itself means transaction consistency
  is only a problem when the same consumers are active across multiple applications
  which can be easily managed thus significantly reducing the possible edge cases
  for consistency problems.
* The distributed commit log ensures that all the materialized views (no matter
  how far away they are) will always eventually become consistent (BASE). 
* Given we have split the transaction logs (distributed commit logs) from the
  materialized view itself we can have the data itself in-process as close to the
  business logic itself (API) and thus enjoy the benefits of database queries that
  are as fast as memory lookups.

Solutions:

* The materialized view is both a cache and an active snapshot of tree (DAG) in
  memory thus it uses cache updating and invalidation messages as a means to
  synchronize itself (publish/subscribe pattern).
* For the very few cases where multiple applications update the same data records
  (partition key) then we can use code and logic to perform a 3-way merge on the
  events that stream into the materialized view thus ensuring correct consistency.
* By selecting an appropriate partition key it is possible to create many small
  materialized views that are largely completely independent and thus creating
  strong data locality and a much smaller memory footprint of the in-memory
  materialized views.

Reference: https://engineering.linkedin.com/distributed-systems/log-what-every-software-engineer-should-know-about-real-time-datas-unifying          
Reference: https://en.wikipedia.org/wiki/Directed_acyclic_graph  

## Caching Locally

ATE is built as a fully distributed commit log that can be validated offline
no matter where its running, hence this presents an immediate advantage for
distributed computing as it gives the ability to replicate a live copy
of the redo log on all clients hence reducing latency on reads to the
speed of the local machine.

Data is invalidated in near real-time as events are streamed from the root
node back to the local nodes. Data is stored on disk which can be either
persistent or temporary depending on needs.

                  .-------------------.
                  |  remote redo-log  |
                  |-------------------|
                  |     TCP Server    |
                  '---------|---------'
                            |
                            |. . . . (async)
                            |
                  .---------|---------.
                  |    TCP Client     |
                  |-------------------|
                  |  local redo-log   |
                  |-------------------|
                  | materialized view |
                  |-------------------|
                  |        APP        |


The local and remote redo-logs are kept in sink at all times over a stateful
TCP connection however the reads and writes operate off the local redo-log.
>>>>>>> origin/main
 
## Materialized Views and Secondary Indexes

ATE deploys Materialized Views inside the library using lambdas and an
automatically updated cache. This allows for complex relational data
structures and models to a coded on top of a stable data stream.

Create your views using a set of helper classes optimized for high performance.

Along with Materialized Views that are dynamically created via table scans
ATE also supports automatic secondary indexes when you use the DaoVec objects
This functionality is enabled by default and can be disabled via the
bootstrap configuration.

Note: Secondary indexes are automatically updated whenever data objects within
      the index are added, removed or updated, there is minimal performance
      impact of this background indexing capability as it takes advantage of
      the data streaming capabilities and some clever programming to
      keep useful indexes in memory when needed

More details are in the code just follow this starting point below:

```rust
pub struct House
{
  pub number: String
}

pub struct MyStreet
{
  pub houses: DaoVec<House>
}

let dio = chain.dio(&session);
let street = dio.load(key);
for house in street.houses.iter(&dio, street.key()) {
    println!("{}", house.number);
}
``` 
   
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

1. It can be compiled down to a single binary with embedded shared configuration
   files.
2. It uses the distributed DNS infrastructure of the Internet to bootstrap itself
   during startup and to validate the various roots for each chain-of-trust.
3. It excels on high throughput network connectivity even when those networks
   display moderate packet loss and latency (a.k.a. The Internet).
4. Data is distributed across the local disks wherever the ATE binary is running
   root nodes, while data integrity during partition events is maintained through
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
   necessary extra setup of the root cluster (e.g. rack awareness, mirror-maker,
   etc..) then run ATE in its "Stateful Mode".
2. Otherwise run in "Stateless Mode".

### Stateful Mode

When operating in this mode the application is both the persistent long term
storage and the worker nodes. All redo-logs are stored in a long term
persistent storage.

The system can be scaled out by added more nodes, when data is needed from
another machine it will make TCP connecting and store a temporary copy locally.

This mode of operation has the following benefits and disadvantages:

* (+1) Its considerable easier to setup, often only requiring a single JAR
  executable to deploy and scale horizontally.
* (+1) Scaling the total system is easier with less components to worry about often
  increasing the capacity is no more than spinning up more nodes. When moving to
  extreme scale with replication all over the world this advantage may not hold
  as the limitations of [CAP theorem](https://en.wikipedia.org/wiki/CAP_theorem)
  become more apparent requiring custom setups of root nodes to fine-tune the
  trade-offs. 
* (-1) As the storage engine runs in the application this making it stateful,
  extra care must be taken when bringing nodes online and taking them offline.
  
Note: Stateful mode is actually a blend of both stateful and stateless nodes. The
DNS records used for bootstrapping the startup will determine which nodes need
to operate the root cluster and which are just plain dumb compute nodes - thus -
it is still possible to scale out an API built onto of Stateful ATE without
worrying about also scaling the stateful elements (i.e. the disks)

### Stateless Mode

When operating in this mode the ATE datachain is running locally in a temporary
folder where it stores all its local redo-logs. All data is persisted on ATE
nodes running on long-term servers.

This mode of operation has the following benefits and disadvantages:

* (+1) Splitting up the scaling components makes it easier to understand the
  performance bottlenecks and scaling limits of the various components.
* (-1) This is a more complex setup from a deployment perspective.
* (-1) Less performance in certain small scale deployments as the data held within
  the distributed commit log may need to travel more distance before it arrives
  at the in-memory materialized view. This disadvantage will diminish as the
  total system is scaled to medium sized deployments.

## Centralized or Distributed Integrity

ATE can operate in two modes when signing and validating the integrity of the
chain, the two modes allow for the owner of the chain to optimize the performance
vs denial-of-service threats. These are how the two modes work...

Distributed Integrity:

- Every writer signs all relevant events fed into the chain-of-trust
- As all relevant events have a signature each one can be validated individually
- There is no need for a central server as anyone can validate the chain-of-trust
- Given each event requires a unqiue cryptographic signature this is quite slow

Centralized Integrity:

- The chain-of-trust is hosted at a central server(s) which is trusted.
- Establishing a connection is secured via a secure cryptographic key exchange
- After making a connection only the first few events are actually signed
- Stateful connections and wire encryption ensure that the integrity is maintained
  during a large number of writes meaning performance is much improved
- If an attacker is able to break into the central server and modify the files
  on the file-system then are may be able to inject their own records however
  they will not be able to read any of the stored data there.

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
  1. DNS Entries that hold the root public keys for the chain-of-trust seeding.
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
                                                                                        
Core weaknesses must be addressed in order to truly strengthen a chain-of-trust
against such attacks - below describes various weakness and how an attacker can
take advantage of them.

### Weakness: Chain-of-trust does not reach the human

     H
     U              |----------|------ -  -
     M      (gap)   | Identity | ... .
     A            ^ |----------|------ -  -
     N            |                      
                attack!
                                   
Misuse Case: Attacker discovers the username/password that a human uses for identity
and  thus pretends to be the legitimate human during login hence they are able to
read and write information that they were not meant to.

**How does ATE help?**

_You must implement strong form of authentication on top of ATE and use this to
generate tokens with the appropriate asymetric crypto keys. ATE does not store
any usernames or passwords (it isnt even designed to store them) thus closing
this attack vector is the responsibility of the application design above ATE_

### Weakness: Chain-of-trust does not reach the information

                                            
     - - -----|------------.                I
     .  . ... | Encryption |      (gap)     N
     - - -----|--------^---'                F
                                     ^      O
                                     |
                                   attack!

Misuse Case: Attacker gains access to the backups of a traditional database, they
export the database away, restore the backup and have read access to everything
that was ever written. 

**How does ATE help?**

_All records within the distributed commit log are encypted with unique encryption
keys that are directly connected to the tail end of the chain-of-trust thus
stealing the database does not allow access to read the data. An attacker must
also break the chain-of-trust in order to get the decryption keys. In fact it
is a perfectly reasonable design to store the distributed commit log in the public
domain without significant confidentiality risks, although one can still keep the
logs secret if they wish._

### Weakness: Chain-of-trust has one or more coarse grained sections

         .----------|----------|          |----------|----------.
         +----------|----------|          |----------|----------|
         +----------|----------|----------|----------|----------+
         +----------|----------|     ^    |----------|----------|
         '----------|----------|     |    |----------|----------'
                                   attack!

**How does ATE help?**

_Ate is built on the principle of zero-trust all the way down to the actual encrypted
records written to the distributed log. This means there is no need for a connection
string or NPA password as a means authorizating system-to-system trust relationships
as the data model itself is protected.
If an attack is able to break into a section of the system they will only be able to
gain read/write access via man-in-the-middle attacks thus preventing many broad and
wide data leakage risks (e.g. leaked NPA passwords or connection strings)._


## Implicit Authority

An interesting case-study is the chain-of-trust that helps establish secure HTTP
connections between web sites and end user devices. E.g. https://www.pcgamer.com/.

If you follow the chain you will find an interesting source of the trust chain.

    DNS -> ACME (LetsEncrypt) -> Certificate (RSA) -> Secure Connection (TLS)
    
Whats interesting is that the ultimate proof of ownership for domains and large
entities is the DNS records themselves. Essentially these are used to generate
certificates, route connections to the correct place and as a register of ownership.

ATE fully embraces DNS as the ultimate source and root of all chain-of-trust for
companies and large entities. It thus contains classes, seeding and generic
authority logic that uses DNS(Sec) to hold and distributed asymmetric encryption
keys (NTRU) that start a chain-of-trust fully independently of the system
designer and operator. This allows for multiple chains-of-trust to be managed,
verified all the way to a refutable legitimate central body and then extend this
relationship all the way down to the encrypted records without any breaks in the
chain.

See this example below for the [Tokera company](https://mxtoolbox.com/SuperTool.aspx?action=txt%3atokauth.tokera.com&run=toolpage)
that publishes its root public key that allows the owner of the private key to
write records to ATE trees that are associated with this company.

Reference: https://mxtoolbox.com/SuperTool.aspx?action=txt%3atokauth.tokera.com&run=toolpage  
Reference: https://letsencrypt.org/how-it-works/  
Reference: https://en.wikipedia.org/wiki/NTRU  

## Fine-Grained Security

As stated in the earlier sections on [chain of trust](#chain-of-trust) and
[implicit authority](#implicit-authority) ATE will maintain and validate a
cryptographically validated chain of trust in memory.

    |  >Crypto-Graph Materiaized View< (in memory) |   .
    |  .----------------------------------.        |   .
    |  |             dns                  |        |   .
    |  |              |                   |        |   .
    |  |      dao----dao                  |        |   |
    |  |              \                   |        |   .
    |  |               dao                |        |   .
    |  |                                  |        |   |
    |  +----------------------------------+        |   .

The data model and design of ATE allows for any particular node within the
chain-of-trust (including the first one) to fork the authority into a new security
domain. This not only allows for each chain-of-trust to operate completely
independently from a security perspective but also allows for these independent
trees to have sub-trees that are themselves completely independently isolated
from a security boundary perspective.

This model creates an interesting side-effect in that roles and access rights
within a chain-of-trust (for instance a company) that is fully validated with
implicit authority (DNS records) to then be carved up into different areas that
are then protected with unique roles with specific access rights.

Having this level of fine grained security can be used to either augment an
existing security architecture with another layer of defence against attack and/or
it can be used to drastically simplify the APIs themselves. For instance - if an
API were to be created using ATE and the supplied token on calls to that API are
then fed into the active scope it becomes much harder to accidentally create some
business logic that leaks information that the user should not have access to as
the ATE framework will throw an exception if it does not have the specific
authority in the token to decrypt the records. This thus reduces the need for
extra validation in the API business logic itself.

## Quantum Resistance

ATE uses asymmetric cryptography that is resistant to attacks from the scaled up
quantum computer(s) of the future. While not a real threat today we must already
build defence against future attacks as the distributed commit log is aimed to live
for very long periods of time, thus attacks in the future will be able to attack
data recorded in the past. Hence it is prudent to select and use algorithms that
are resistant to quantum attacks and ideally the algorithms selected must also
exhibit forward secrecy where possible. This is especially important as it is
estimated at the time of writing that capable quantum based attacks on
cryptographic will be possible in the next 5 years.

The following asymmetric cryptography have been shown mathematically to be highly
vulnerable to such attacks:

* ring-LWE algorithms
* RSA-1024, RSA-2048, RSA-4096
* ECC-256, ECC-521
* Diffie-Hellman
* Elliptic curve Diffie-Hellman

ATE adopts the principle of superencryption (cascading encryption with the rule
of two) using distinctly different algorithms which ideally reside in different
cryptographic groups. The idea behind this extra computation, complexity and key
size is that if a weakness is found in one of the ciphers in the future then at
least the second cipher will protect the customer data for a reasonable amount
of time until a fix can be rolled out to eliminate the systemic weakness. Given
ATE is highly dependent on cryptography for its authentication and authorization
models this is deemed an acceptable cost.

`
Note: When using cascading encryption separate encryption keys are generated using
secure random number generators.
`

ATE uses two asymmetric signature algorithms to prevent breach of integrity (a.k.a.
unauthorized writes):

- **Falcon** - _lattice-based _
- **Rainbow** - _multivariable polynomial_

ATE uses two asymmetric encryption algorithms to prevent breach of confidentially
(a.k.a. unauthorized reads):

- **NTRU** - _lattice-based (shortest vector problem)_
- **NewHope** - _lattice-based (ring learning with errors)_

ATE uses symmetric encryption to encrypt the actual data itself with the keys hidden
behind the earlier asymmetric encryption (this is done for performance reasons
through the reuse of the faster symmetric encryption when within the same security
boundary):

- **AES256** - _equivalent to AES128 on quantum computer_
- **AES512** - _equivalent to AES256 on quantum computer_

`
Note: For the observant reader this means if AES is broken in the future then
everything is broken no matter what asymmetric signature or encryption algorithms
as this would allow for reading of all the data without the need to break asymmetric
cryptographic. AES is currently deemed post quantum resistant when doubling
the key size besides this no viable alternatives exist at the time of design.
`

All of these algorithms are candidates for NIST post quantum cryptography:  
https://en.wikipedia.org/wiki/Post-Quantum_Cryptography_Standardization#cite_note-20

XMSS-MT and NewHope provide forward secrecy

Reference: https://en.wikipedia.org/wiki/Grover%27s_algorithm  
Reference: https://en.wikipedia.org/wiki/Multiple_encryption  
Reference: https://blog.cryptographyengineering.com/2012/02/02/multiple-encryption/  
Reference: https://en.wikipedia.org/wiki/Post-Quantum_Cryptography_Standardization#cite_note-20  
Reference: https://en.wikipedia.org/wiki/Post-quantum_cryptography    
Reference: https://nvlpubs.nist.gov/nistpubs/ir/2016/NIST.IR.8105.pdf    
Reference: https://en.wikipedia.org/wiki/NTRU  
