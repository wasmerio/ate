ate-2.0
=======

- are-2.0 stores all its persistent data in redo-logs
- redo-logs are broken into a header file and body file stored locally
- the logs are stored in a directory with a specific naming convention and loaded on demand
- each log file represents a linear chain-of-trust as a tree of accepted events
- chains can be linked together creating circular dependencies
- subscription management is represented as stateful TCP connections
- subscription management is coarse-grain at a fixed number per broker
- subscription management uses relays to keep connection limit to less than 10k
- actors on a chain can play different roles
  + archive - read-only copy of a set of chains that are no longer active
  + legacy - copy of a chain that is no longer active on this machine
  + master - currently owns an active persistent copy of the chain
  + replica - has a persistent copy of the chain in a particular region of the globe
  + client - has a temporary local copy of the chain that is kept up-to-date by the master
  + relay - design time list of actors that will simply relay traffic without explicitely storing the chain
- master topology is shifted-left into a configuration file that is design time which seeds the chain discovery workflow
- chains are mapped to masters via a convention on the hash of the partition key for the first node in the chain
- redo-logs are materialized into in-memory databases using materializers
- when accessing the materialized view a context is supplied that holds user defined structured data
- each event has a header that includes structured metadata about the data held in the event
- events follow generics hence they are all strongly typed however the following defaults are available
  + key-value - events that hold a string in the header and a string in the body
  + file-system - events that can represent a file-system
  + ledger - events that represent a fiancial ledger of transactions
- materializers turn the raw data of the files into indexed structured data
- materializers are pipelined together to perform the following functions
  + acceptance - step that determines if an event is accepted into the chain or not
  + load - turns the binary-data of an event into a structured data object
  + flip - recreates the redo-log during a flip operation working backwards from newest-to-oldest
- the following materializers are available out-of-the-box:
  + accept-all - simply accepts all events that it receives
  + master-key-validator - accepts an event if it includes a signature by a rotatable master key
  + trust-delegation - accepts an event if its parent has delegated trust to it via verifiable signatures
  + published-ownership - accepts an event if a DNS queried public key matches the signature of the event
  + structured-logic - denies the acceptance of an event if it does not pass a series of callback operations
  + throttle - denies the acceptance of an event if it exceeds specific measured rate (using a heartbeat events as a carrier)
  + confidentiality - encrypts and decrypts binary-data during the load function using key material supplied in the current context
  + serializer - turns the binary data to-and-from a structured data object
  + condense-by-header - removes events based off callbacks that take the header metadata and the new materialized view as input
  + condense-by-tombstone - removes events that have been tombstoned
  + worker - executes work by calling a callback with a supplied context when an event of a particular type is received
- multiple heartbeats are transmitted by the "master" node on a regular bases as "signed events" of particular types
- all participants subscribed to a chain will perform a condense operation when the heartbeat reaches a threshold
- condense operations are executed by flipping the front and back buffers of the chain in response to a "flip" heartbeat
- during a condense operation both buffers receive and validate events however the active buffer only swaps after
  it is confirmed that all data is written to disk as new header and body files
- uses derivation to mark code methods as business logic that can be invoked both client-side and server-side
- includes a set of default methods that allow the user to read and modify all structured data in a chain
- provides token support that includes the context required for confidentialy of body data

ate-fs
======
- methods can be annotated as structured files in an emulated file-system
- provides a series of optional external interfaces to the chains
   + 'fuse' to expose files that represent the different annotated methods
   + REST API's for all annotated methods for easy consumption from websites