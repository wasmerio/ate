use wasmer_wasi::os::TtyOptions;
use wasmer_wasi::{
    WasiRuntimeImplementation,
    WasiTtyState, VirtualBus, VirtualNetworking, WasiEnv
};
use std::io;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct SshRuntime {
    pub stdout: mpsc::Sender<Vec<u8>>,
    pub stderr: mpsc::Sender<Vec<u8>>,
    pub flush: mpsc::Sender<()>,
    pub tty: TtyOptions,
}

impl WasiRuntimeImplementation
for SshRuntime
{
    /// For WASI runtimes that support it they can implement a message BUS implementation
    /// which allows runtimes to pass serialized messages between each other similar to
    /// RPC's. BUS implementation can be implemented that communicate across runtimes
    /// thus creating a distributed computing architecture.
    fn bus(&self) -> &(dyn VirtualBus<WasiEnv>) {
        self
    }

    /// Provides access to all the networking related functions such as sockets.
    /// By default networking is not implemented.
    fn networking(&self) -> &(dyn VirtualNetworking) {
        self
    }

    /// Writes output to the SSH pipe
    fn stdout(&self, data: &[u8]) -> io::Result<()> {
        let data = match self.tty.line_feeds() {
            true => {
                data.to_vec()
                    .into_iter()
                    .flat_map(|a| match a {
                        b'\n' => vec![ b'\r', b'\n' ].into_iter(),
                        a => vec![ a ].into_iter()
                    })
                    .collect::<Vec<_>>()
            },
            false => data.to_vec()
        };
        blocking_send(&self.stdout, data)
    }

    /// Writes output to the SSH pipe
    fn stderr(&self, data: &[u8]) -> io::Result<()> {
        let data = match self.tty.line_feeds() {
            true => {
                data.to_vec()
                    .into_iter()
                    .flat_map(|a| match a {
                        b'\n' => vec![ b'\r', b'\n' ].into_iter(),
                        a => vec![ a ].into_iter()
                    })
                    .collect::<Vec<_>>()
            },
            false => data.to_vec()
        };
        blocking_send(&self.stderr, data.to_vec())
    }

    /// Flushes the data down the SSH pipe
    fn flush(&self) -> io::Result<()> {
        blocking_send(&self.flush, ())
    }

    /*
    /// Tell the process to exit (if it can)
    async fn exit(&self) {
        let mut handle = self.handle.clone();
        let _ = handle.close(self.channel).await;
    }
    */

    fn tty_get(&self) -> WasiTtyState {
        WasiTtyState {
            cols: self.tty.cols(),
            rows: self.tty.rows(),
            width: 800,
            height: 600,
            stdin_tty: true,
            stdout_tty: true,
            stderr_tty: true,
            echo: self.tty.echo(),
            line_buffered: self.tty.line_buffering(),
            line_feeds: self.tty.line_feeds(),
        }
    }

    fn tty_set(&self, tty_state: WasiTtyState) {
        self.tty.set_cols(tty_state.cols);
        self.tty.set_rows(tty_state.rows);
        self.tty.set_echo(tty_state.echo);
        self.tty.set_line_buffering(tty_state.line_buffered);
        self.tty.set_line_feeds(tty_state.line_feeds);
    }
}

impl VirtualBus<WasiEnv>
for SshRuntime
{

}

impl VirtualNetworking
for SshRuntime
{

}

fn blocking_send<T>(tx: &tokio::sync::mpsc::Sender<T>, mut data: T) -> io::Result<()> {
    let mut wait_time = 0u64;
    loop {
        // Try and send the data
        match tx.try_send(data) {
            Ok(_) => {
                return Ok(());
            }
            Err(tokio::sync::mpsc::error::TrySendError::Full(returned_msg)) => {
                data = returned_msg;
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
        }

        // Linearly increasing wait time
        wait_time += 1;
        let wait_time = u64::min(wait_time / 10, 20);
        std::thread::park_timeout(std::time::Duration::from_millis(wait_time));
    }
}
