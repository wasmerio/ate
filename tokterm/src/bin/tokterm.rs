#![allow(unused_imports)]
use clap::Parser;
use raw_tty::GuardMode;
use std::io::Read;
use term_lib::api::*;
use term_lib::console::Console;
use tokio::io;
use tokio::select;
use tokio::sync::watch;
use tokterm::utils::*;
use tracing::{debug, error, info, warn};
#[cfg(unix)]
use {
    libc::{c_int, tcsetattr, termios, ECHO, ECHONL, ICANON, TCSANOW},
    std::mem,
    std::os::unix::io::AsRawFd,
};

#[allow(dead_code)]
#[derive(Parser)]
#[clap(version = "1.0", author = "Tokera Pty Ltd <info@tokera.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
    /// Logs debug info to the console
    #[clap(short, long)]
    pub debug: bool,
    /// Determines which compiler to use
    #[clap(short, long, default_value = "default")]
    pub compiler: term_lib::eval::Compiler,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();

    // Set the panic hook that will terminate the process
    let mut tty = set_mode_no_echo();
    let old_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        set_mode_echo();
        old_panic_hook(panic_info);
        std::process::exit(1);
    }));

    // Initialize the logging engine
    tokterm::utils::log_init(opts.verbose, opts.debug);

    // Set the system
    let (tx_exit, mut rx_exit) = watch::channel(false);
    let sys = tokterm::system::SysSystem::new(tx_exit);
    let con = Arc::new(sys.clone());
    term_lib::api::set_system_abi(sys.clone());
    let system = System::default();

    // Read keys in a dedicated thread
    let (tx_data, mut rx_data) = tokio::sync::mpsc::channel(term_lib::common::MAX_MPSC);
    system.fork_dedicated(move || async move {
        let mut buf = [0u8; 1024];
        while let Ok(read) = tty.read(&mut buf) {
            let buf = &buf[..read];
            unsafe {
                let _ = tx_data
                    .send(String::from_utf8_unchecked(buf.to_vec()))
                    .await;
            }
        }
    });

    // Now we run the actual console under the runtime
    let compiler = opts.compiler;
    sys.block_on(async move {
        let location = "wss://localhost/".to_string();
        let user_agent = "noagent".to_string();
        let mut console = Console::new(location, user_agent, compiler, con);
        console.init().await;

        // Process data until the console closes
        while *rx_exit.borrow() == false {
            select! {
                data = rx_data.recv() => {
                    if let Some(data) = data {
                        console.on_data(data).await;
                    } else {
                        break;
                    }
                }
                _ = rx_exit.changed() => {
                }
            }
        }

        // Clear the screen
        let _ = system.print("\r\n".to_string()).await;
    });

    // We are done
    set_mode_echo();
    Ok(())
}
