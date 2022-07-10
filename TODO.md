# Bugs to fix

- The coin splitting goes wild when there is too much money in a wallet
  (better to split the coins by the exact amount and simplify the design).
  Also need to be super careful for lost coins - apparently some go missing
  during either the split or the combine, not sure which but I suspect the combine
- The contracts are not visible in Wasmer domains/groups even though one has
  been added

# Features to implement

- The /.app folder needs to save its files to a temporary file or directory
  on the real machine so that it saves memory.
- Cached compiled objects need to save their files to temporary files so
  that it saves memory.

# Fun stuff

- getting this working will be cool
  https://crates.io/crates/egui
  https://www.egui.rs/#demo

- it needs glow integrated with wasmer-bus-webgl
  https://crates.io/crates/glow

- otherwise this demo is also a nice one
  https://www.chinedufn.com/3d-webgl-basic-water-tutorial/

- make this work...
  https://crates.io/crates/macroquad

  so this works...
  https://github.com/Gerstacker/macroquad-forestfire/blob/main/src/main.rs

# Done

- Logging in will mount the same folder twice which will show double the files
  so to fix this mounts should override mounts
- There is a panic on the instance shell functionality which needs to be fixed
- Some times the instance process in Wasmer outputs giberish - likely cause
  of this is a partial IV received and then corruption on the decrypted data.
  Reason for the partial received IV is because web sockets are not guaranteed
  to send per-packet so need to add in some length pre-qualifier or just
  add buffering
- Need to fix the browser tests for the MAC - there is now some testing suite
  that should work and thus allow for some debugging
- Firing off web sockets to unreachable ports freezes the proces. This is the
  case for instances 'deploy' for instance when they dont go to the correct port.

