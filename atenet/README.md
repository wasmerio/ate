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
