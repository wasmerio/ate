#![allow(unused_imports)]
use clap::Parser;
use raw_tty::GuardMode;
use std::io::Read;
use std::sync::Arc;
use wasmer_os::api::*;
use wasmer_os::console::Console;
use tokio::io;
use tokio::select;
use tokio::sync::watch;
use wasmer_term::wasmer_os::bin_factory::CachedCompiledModules;
use wasmer_term::utils::*;
use tracing::{debug, error, info, warn};
#[cfg(unix)]
use {
    libc::{c_int, tcsetattr, termios, ECHO, ECHONL, ICANON, TCSANOW},
    std::mem,
    std::os::unix::io::AsRawFd,
};

#[allow(dead_code)]
#[derive(Parser)]
#[clap(version = "1.0", author = "Wasmer Inc <info@wasmer.io>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
    /// Logs debug info to the console
    #[clap(short, long)]
    pub debug: bool,
    /// Determines which compiler to use
    #[clap(short, long, default_value = "default")]
    pub compiler: wasmer_os::eval::Compiler,
    /// Location where cached compiled modules are stored
    #[clap(long, default_value = "~/wasmer/compiled")]
    pub compiler_cache_path: String,
    /// Uses a local directory for native files rather than the published ate chain
    #[clap(long)]
    pub native_files_path: Option<String>,
    /// Runs a particular command after loading
    #[clap(index = 1)]
    pub run: Option<String>,
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
    wasmer_term::utils::log_init(opts.verbose, opts.debug);

    // Set the system
    let (tx_exit, mut rx_exit) = watch::channel(false);
    let sys = wasmer_term::system::SysSystem::new(opts.native_files_path, tx_exit);
    let con = Arc::new(sys.clone());
    wasmer_os::api::set_system_abi(sys.clone());
    let system = System::default();

    // Read keys in a dedicated thread
    let (tx_data, mut rx_data) = tokio::sync::mpsc::channel(wasmer_os::common::MAX_MPSC);
    system.fork_dedicated_async(move || async move {
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

    // Build the compiled modules
    let compiled_modules = Arc::new(CachedCompiledModules::new(Some(opts.compiler_cache_path)));

    // If a command is passed in then pass it into the console
    let location = if let Some(run) = opts.run {
        format!("wss://localhost/?no_welcome&init={}", run)
    } else {
        format!("wss://localhost/")
    };

    // Now we run the actual console under the runtime
    let fs = wasmer_os::fs::create_root_fs(None);
    let con = con.clone();
    let compiler = opts.compiler;
    sys.block_on(async move {
        let user_agent = "noagent".to_string();
        let mut console = Console::new(
            location,
            user_agent,
            compiler,
            con.clone(),
            None,
            fs,
            compiled_modules,
        );
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
        let _ = con.stdout("\r\n".to_string().into_bytes()).await;
    });

    // We are done
    set_mode_echo();
    Ok(())
}
