# WASM Fuse Bus

The WASM Fuse Bus allows WebAssembly modules to expose a file system
to any runtime that supports the WASM General Purpose Bus

# Reference Implementation

A reference implementation exists here:
https://github.com/wasmerio/ate/blob/master/wasmer/src/bus/main.rs

However this remains quite a low-level integration, once macros are
defined that can emit the code for this interface it will be superceded
by a similar implementation.

# Backend Implementations

In order to implment this BUS on your runtime one needs to chain to
the ABI exposed in this library and implement the functions.

For a reference implementation see below:

https://github.com/wasmerio/ate/tree/master/wasmer_term/src/bus

# Testing

You can test your WASI program by uploading it to wapm.io and then heading over to the Wasmer Shell

https://wasmer.sh