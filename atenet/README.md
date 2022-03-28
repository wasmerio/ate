# Design

- MIO uses wasm-bus-mio as an interface to the operating system
- atenet will be a distributed vswitch built on top of ATE as a route table
- wasm-bus-mio must implement TCP/IP stack in atenet
  (https://github.com/smoltcp-rs/smoltcp)
- The 'tok' binary will be extended with tun/tap support so that users
  can connect to arbitary networkings from their machines
- The browser based version will connect to the vSwitch using web sockets
- x86 implementations will connect using encrypted (AES) UDP packets
- Packets are signed using a secret stored in the ate redo log for an instance


```

                                                         (tun/tap)
                 browser                                    tok
                    |                                        |
                  (wss)                 .---------.        (udp)
                    |                   | atesess |          |
               .----|----.              |----|----|     .----|----.
               | atenet  |- - (udp) - - | atenet  | - - | atenet  |
               '----|----'              '----|----'     '----|---'
                    \                        |               /
                     \                  (subscribe)         /
                      \                      |             /
                      .------------------------------------.
                      |      tokdb - chain-of-trust        |
                      '------------------------------------'

```

# MVP

The first version is a dump switch (i.e. all data gets broadcast) - later versions will
use ARP snooping to only send data to other nodes that are meant to get the data

No default gateway will be available on the first version hence it is not possible to
make internet calls outside of the closed network

There is no DHCP server in the first version and hence all IP addresses need to be statically
generated which will be done using a hash

# Security

The security of the distributed networking is built upon the strong security of ATE
itself however there are few specifics that make this viable

1. The 'atenet' binaries need to have access to the chains in order to read the
   configuration hence they reuse the master keys that are also used by 'ateses'
2. Creating an 'instance' in Tokera is the same chain also used for the vSwitch
   and hence there is no need to associate a network with a instance.
3. Exporting access to a vSwitch gives an access key - more than one can be
   exported thus allowing for easier issuance and revoking for VPN users.
4. Packets are all stateless and validated individually.

# Packet Format

All packets are encrypted and signed using the access token.

- The AES encryption key is derived from the access token with a fixed seed.
- The SHA256 signing key is derived from the access token using a different fixed seed.
- Each packet is prefixed with a 16-byte chain-of-trust that defines the chain
  that contains the configuration.

# vSwitch

- The vSwitch stores its state in an ATE redo log
- All nodes currently participating in a vSwitch are added to a collection
- Only the nodes themselves can add themselves or remove themselves from a collection
- Upon receiving a packet from another node that is for a vSwitch thats not
  currently active (i.e. there are not client connections) then it will remove
  itself from the chain-of-trust.
- vSwitches will send UDP packets for anything they receive to all other vSwitches
  that are running in other nodes (flood)
  Note: In order to avoid loops any packets received from other vSwitches are not
  themselves sent to other vSwitches, only the Tun/Tap devices.
- Tun/Tap devices are added to each vSwitch to represent clients that remotely connect
- All running WAPM instances also add a Tun/Tap device to the local vSwitch
- All packets are flooded (no multicast/unicast ARP snooping)
- Client can connect using web-socket connections
- Web socket connections will drop packets (random chance) when they are saturated
- UDP connections will remain active for 60 seconds after receiving data
- wasm-bus connections will remain active as long as they have active WASM programs
- The vSwitch will maintain diagnostic information in the redo log for everything its managing

# vRouter

In order for a workload to talk to another workload thats running in a different vSwitch
they need to be able to route packets between different subnets. For this a vRouter is
needed.

In the chain-of-trust is a list of routes to other vSwitches that is updated using
the 'tok' command line utility - this utility will also associate an access key with
the route for security reasons. The list of 'active' access keys is also maintained in
the chain of trust for others to use and modified using the 'tok' util.

Each vSwitch will create a 'ethereal interface' that is only visible to local clients
connected to the vSwitch (thus preventing loops). This ethereal interface represents
the vRouter and holds a real IP4/IP6 address. This device acts as a 'default router'
and will examine all IP packets that it receives then makes a decision on what to do.

Upon receiving a packet for a destination IP address that's on another vSwitch then atenet
will load that other vSwitch locally and feed the packet into this network using the
associated 'default router' of the secondary vSwitch after the MAC addresses are
substituted.

# DHCP Server

Implementing a DHCP server should use a 'ethereal interface' and store its address
registration table in the distributed redo log.

# DNS Server

DNS will simply point to 8.8.8.8 thus avoiding the need to implement this.

# VPN

The 'tok' binary will be enhanced to support Linux based tun/tap devices that connect
to a vSwitch using standard UDP packets. The VPN should send out periodic ARP packets
every 15 seconds to ensure that it stays subscribed in the vSwitch.

The access key for a particular network is stored in the chain-of-trust and thus there
is no need for an authentication protocol.

# Internet

The default router is further enhanced so that any packets destined for Internet addresses
are sent out on the 'atenet' public IP address using a unique source port (NAT) that
is used to process the response.

# ARP Snooping

Later in order to get better performance and reduce the overhead on large vSwitches it
will become nessasary to implement an ARP scooping implementation that once a MAC
is known to reside on a specific tun/tap and/or remote vSwitch then it will only be
sent to this address.

# Multicast Snooping

As functionality evolves and especially for IPv6 the need for multicast snooping will
be required. For this the vSwitch will also listen for ICMP packets that subscribe to
particular multi-cast addresses. Upon which sending to a particular multi-cast address
will mean the packets are only sent to those that subscribed.
