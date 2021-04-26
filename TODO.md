```
bugs   Found Bugs
       + Modified files are still showing zeros at the end
       + Changing the permissions on a parent is not reflected in the children as inherited
         rows that are encrypted do not automatically gain the new keys of the parent. Will
         need to store a read-key in the parent which the children use when they are in
         inheritance mode
       + Still need to fix the writing actions when a user attempts to change an object
       + Fixed a bug where files opened with truncate flag were not actually truncating
0.5.1  Group access rights
       D+ AteAuth requires group access rights that also get added to the token
       D+ Connect up the 'chmod' commands to real commands in AteAuth so that
       D  actual ATE data object access rights reflect the linux permissions
       D+ All users and groups should be contained to primary keys within 32bits so
       D  that they work with uid and gid with the highest bit set.
        + Lists files and folders should ignore those that you do not have access to
          rather than throwing an error
        + File-systems should record the correct uid and gid within ATE but change it
          to the actual user when mounted if it matches. Using the chown command
          should allow the object to be given to other users and other groups.
        + When attempting to access a group that is not this group it should attempt
          to get the latest permissions from the authentication server.
0.5.2   Compacting chains
        + AteDb should periodically compact itself without breaking things
        + Events that are streamed to a compacted chain that predate the
          compaction should be dropped.
        + Deleting all the entries in a chain should also destroy the chain
0.5.3   Linked File-System
        + Any folder created within AteFS should be able to 'link' with another
          file-system using the 'atefs link {remote}' commands.
        + The working directory is taken as the folder you wish to union however
          you can specify this in the command line as an argument instead.
        + Internally AteFS downloads and operates on a different file-system
          as if it had been directly mounted.
        + Hook the 'ln -s' file-system command to make this more seamless
        + Removing the folder destroys the link within killing the remote chain itself
0.5.4   Union File-Systsem
        + Any linked folder 'atefs link' can be forked instead of linked using
          the 'atefs fork {remote}' command.
        + Union'ed folders behave like linked folders except all changes made are
          stored in the 'local' chain instead of the 'remote' chain
        + Only read-only access is required to the 'remote' chain'
        + Local files and folders take preference over remote files
        + Deleted files and folders use a whiteout marker (.wh.{file}).
0.7.0   Docker imports
        + AteDocker needs to be created that hosts imported docker files
          on demand as they are requested.
        + AteDocker will run at 'tcp://docker.tokera.com/'
        + Docker credentials should be passed in the command-line, only public
          containers are supported at Tokera - private containers require
          one to run their own instance of AteDocker.
        + The command to run on the docker image is stored in a file called 'init'
          at the root of the file-system (unless the file already exists)
        + Users can load and modify docker images simply by using the 'atefs union'
          and 'atefs fork' commands.
0.9.4   Process Dispatch Point
        + So called PDP 'process dispatch points' can be created within AteFS which
          when running on a specific machine will serve RPC(Remote Process Calls)
        + The dispatch point uses a hardware identity scan locally to determine
          if it is the owner of the PDP.
        + Every PDP has a unique user attached to it that has specific access
          rights - the authentication is a combination of a secret embedded in the parent
          file-system plus the hardware identity hash.
        + The PDP waits for commands and then executes a process locally streaming
          the results back to the caller - it follows these steps.
          1. bind all mount points to the folder
          2. wait for a command to be received over ATE
          3. chroot to the folder and execute the command
          4. stream the stdout and stderr back to the caller
          5. if no other processes are running then clean up the mount points
        + If the folder that has been turned into an PDP holds a /init file then
          this file is launched automatically (restarting it if it fails)
0.9.5   Remote Process Calls
        + All executables after a PDP on mounts that are not running as a server
          are replaced by a fake executable that proxies the command to the server.
        + Remote operators should be able to simply CHROOT to the folder to perform any
          action as if they were on the remote server/client.
0.8.0   Ate Bootloader
        + Bootloader created and stored in the public ate repository that others
          can download onto USB sticks.
        + Bootloader does the following...
          1. Creates a ext4 file system across all the block devices
          2. Downloads the file-system locally using AteFS
          3. AteFS runs on a specfied file system
          4. Mounts all the auxillary file-systems
          5. Creates a PDP on the folder (if one does not exist)
0.9.0   Tokera Coins
        + Create wallets for accounts in Tokera
        + Create PayPal exchange for wallets
        + Add contracts
0.10.0  Rentable Baremetal
        + Ate bootloader that goes into an advertising state which publishes the
          machine and its specs on an open market.
        + Buyers can run test loads on an advertised machine to see if they are
          happy with how it respondes before they commit
        + Once purchased the server will chain over to another file-system which
          the renter has specified
        + PDP will take care of automatically running stuff when the machine boots
        + Remote operations is all done via the AteFS and PDP
0.11.0  9P Emulation
        + Any mount point within the file system can be attached via the
          single UNIX socket - should be able to remount atefs endpoints
          using a 9p mount command either from within or outside a VM.
0.12.0  Rentable Hypervisor
        + Another Ate bootloader but this one will launch bootloaders as virtual
          machines that carve up the machine.
        + KVM workloads can run off the 9P emulated file system directly
        + Block devices point to files on an EXT4 partition however the
          earlier bootloader is used in combinated with 9P
        + PDP will take care of automatically running stuff when the machine boots
        + Remote operations is all done via the AteFS and PDP
0.13.0  Virtualized Networking
        + All networking are abstracted behind virtual machines that are attached
          to a local only bridge.
        + Networks are peered together via configuration and firewall enforced IPv6
          tunnels with 'ipsec'.
        + External connectivity is managed by load-balancer on the public IP address
```

parked
======

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