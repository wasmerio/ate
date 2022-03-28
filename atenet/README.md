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

# vSwitch

- The vSwitch stores its state in an ATE redo log
- All nodes currently participating in a vSwitch are added to a collection
- Only the nodes themselves can add themselves or remove themselves from a collection
- Upon receiving a packet from another node that is for a vSwitch thats not
  currently active (i.e. there are not client connections) then it will remove
  itself from the chain-of-trust.
- vSwitches will send UDP packets for anything they receive to all other vSwitches
  that are running in other nodes (flood)
- Tun/Tap devices are added to each vSwitch to represent clients that remotely connect
- All running WAPM instances also add a Tun/Tap device to the local vSwitch
- All packets are flooded (no multicast/unicast ARP snooping)
- Client can connect using web-socket connections
- Web socket connections will drop packets (random chance) when they are saturated
- UDP connections will remain active for 60 seconds after receiving data
- wasm-bus connections will remain active as long as they have active WASM programs
- The vSwitch will maintain diagnostic information in the redo log for everything its managing