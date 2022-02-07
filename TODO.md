# Bugs to fix

- Logging in will mount the same folder twice which will show double the files
  so to fix this mounts should override mounts
- Firing off web sockets to unreachable ports freezes the proces. This is the
  case for instances 'tok' for instance when they dont go to the correct port.
- There is a panic on the instance shell functionality which needs to be fixed
- The coin splitting goes wild when there is too much money in a wallet
  (better to split the coins by the exact amount and simplify the design).
  Also need to be super careful for lost coins - apparently some go missing
  during either the split or the combine, not sure which but I suspect the combine
- Some times the instance process in Tokera outputs giberish - likely cause
  of this is a partial IV received and then corruption on the decrypted data.
  Reason for the partial received IV is because web sockets are not guaranteed
  to send per-packet so need to add in some length pre-qualifier or just
  add buffering
- The contracts are not visible in Tokera domains/groups even though one has
  been added
- Need to fix the browser tests for the MAC - there is now some testing suite
  that should work and thus allow for some debugging

# Features to implement

- The /.app folder needs to save its files to a temporary file or directory
  on the real machine so that it saves memory.
- Cached compiled objects need to save their files to temporary files so
  that it saves memory.

# Fun stuff

- make this work...
  https://crates.io/crates/macroquad

  so this works...
  https://github.com/Gerstacker/macroquad-forestfire/blob/main/src/main.rs
