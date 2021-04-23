```
0.5.1   Group access rights
        + AteAuth requires group access rights that also get added to the token
        + Connect up the 'chmod' commands to real commands in AteAuth so that
          actual ATE data object access rights reflect the linux permissions
0.5.2   Very large file-system support
        + Files larger than 20MB should be stored in a seperate chains
        + Files larger than 200MB should be stored in extents that each have
          their own chains
0.5.3   Compacting chains
        + AteDb should periodically compact itself without breaking things
        + Events that are streamed to a compacted chain that predate the
          compaction should be dropped.
        + Deleting all the entries in a chain should also destroy the chain
0.5.4   Snapshots and References
        + Taking a snapshot of a file system should cause the chain to rotate
          which makes it immutable (need an emulated folder to restore and
          create those snapshots)
0.6.0 - Remote Symbolic links
        + Need to be able to create a symbolic link to another remote file system
          (e.g. ln -s tcp://ate.tokera.com/myfs myfs)
0.6.1   Advanced File-Systems
        + Flag needed in AteFS that enables the 'advanced mode' (default: on)
        + The root of the file-system is the users folder
        + Add a symbolic link over to an account file system /acc
        + Add a symbolic link over to a public file system /pub -> https://ate.tokera.com/pub
          (this file-system is owned by Tokera with access granted on specific folders)
0.7.0   Docker imports
        + AteDocker needs to be created that hosts imported docker files
          on demand as they are requested.
        + Docker credentials should be stored in the authentication server
          and used by AteDocker with appropriate encryption when the
          image is private
        + Importing a docker image will create snapshot chains for each step in
          the docker image
        + Users can load and modify docker images simply by creating a symbolic link to
          its address on docker hub.
0.8.0  Reverse Proxy
        + When mounting a file-system within an AteFS location then a reverse
          proxy should be started that then makes it visible for anyone who
          accesses the file-system.
0.8.1  Remote Process Call
        + AteFs should have a mode that can be activated that allows for RPC.
        + An RPC is an service endpoint that will execute a process locally
          which reads from stdin and writes to stdout
        + When running a process on one copy of AteFS it should then be possible
          to see the output on another using this method.
0.9.0  Ate Bootloader
        + AteBoot needs to be created which holds an initrd.img, token and URL
          of a file-system to boot to.
        + On the public file-system a series of images should be created that
          gives different flavors of systems one can boot too.
        + One of the core bootloaders should do the following
          1. Downloads the file-system locally using AteFS
          2. AteFS runs with the RPC mode enabled
          3. Mounts all the auxillary file-systems
          4. Runs reverse proxies on all the mount points
0.10.0  Tokera Coins
        + Create wallets for accounts in Tokera
        + Create PayPal exchange for wallets
        + Add contracts
0.11.0  Rentable Server
        + Ate bootloader that goes into an advertising state which publishes the
          machine and its specs on an open market.
        + Buyers can run test loads on an advertised machine to see if they are
          happy with how it respondes before they commit
        + Once purchased the server can run one of more virtual machines off
          customer supplied file-systems with init processes.
        + Test it as a virtual machine.
```

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