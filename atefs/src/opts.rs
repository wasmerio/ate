#[allow(unused_imports, dead_code)]
use log::{info, error, debug};
use ate::prelude::*;
use ate_auth::opts::*;
use url::Url;

use ate::compact::CompactMode;

use clap::Clap;

#[derive(Clap)]
#[clap(version = "1.6", author = "John S. <johnathan.sharratt@gmail.com>")]
pub struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[allow(dead_code)]
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
    /// URL where the user is authenticated
    #[clap(short, long, default_value = "ws://tokera.com/auth")]
    pub auth: Url,
    /// No authentication or passcode will be used to protect this file-system
    #[clap(short, long)]
    pub no_auth: bool,
    /// Token used to access your encrypted file-system (if you do not supply a token then you will
    /// be prompted for a username and password)
    #[clap(short, long)]
    pub token: Option<String>,
    /// Token file to read that holds a previously created token to be used to access your encrypted
    /// file-system (if you do not supply a token then you will be prompted for a username and password)
    #[clap(long)]
    pub token_path: Option<String>,
    /// No NTP server will be used to synchronize the time thus the server time
    /// will be used instead
    #[clap(long)]
    pub no_ntp: bool,
    /// NTP server address that the file-system will synchronize with
    #[clap(long)]
    pub ntp_pool: Option<String>,
    /// NTP server port that the file-system will synchronize with
    #[clap(long)]
    pub ntp_port: Option<u16>,
    /// Logs debug info to the console
    #[clap(short, long)]
    pub debug: bool,
    /// Determines if ATE will use DNSSec or just plain DNS
    #[clap(long)]
    pub dns_sec: bool,
    /// Address that DNS queries will be sent to
    #[clap(long, default_value = "8.8.8.8")]
    pub dns_server: String,
    /// Indicates if ATE will use quantum resistant wire encryption (possible values are 128, 192, 256).
    /// The default is not to use wire encryption meaning the encryption of the event data itself is
    /// what protects the data
    #[clap(long)]
    pub wire_encryption: Option<KeySize>,
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Clap)]
pub enum SubCommand {
    /// Mounts a local or remote file system
    #[clap()]
    Mount(OptsMount),
    /// Users are needed to access any remote file systems
    #[clap()]
    User(OptsUser),
    /// Groups are collections of users that share same remote file system
    #[clap()]
    Group(OptsGroup),
    /// Tokens are needed to mount file systems without prompting for credentials
    #[clap()]
    Token(OptsToken),
}

/// Mounts a particular directory as an ATE file system
#[derive(Clap)]
pub struct OptsMount {
    /// Path to directory that the file system will be mounted at
    #[clap(index=1)]
    pub mount_path: String,
    /// Name of the file-system to be mounted (e.g. myfs).
    /// If this URL is not specified then data will only be stored in a local chain-of-trust
    #[clap(index=2)]
    pub remote_name: Option<String>,
    /// URL where the data is remotely stored on a distributed commit log.
    #[clap(short, long, default_value = "ws://tokera.com/db")]
    pub remote: Url,
    /// (Optional) Location of the local persistent redo log (e.g. ~/ate/fs")
    /// If this parameter is not specified then chain-of-trust will cache in memory rather than disk
    #[clap(long)]
    pub log_path: Option<String>,
    /// Determines how the file-system will react while it is nominal and when it is
    /// recovering from a communication failure (valid options are 'async', 'readonly-async',
    /// 'readonly-sync' or 'sync')
    #[clap(long, default_value = "readonly-async")]
    pub recovery_mode: RecoveryMode,
    /// User supplied passcode that will be used to encrypt the contents of this file-system
    /// instead of using an authentication. Note that this can 'not' be used as combination
    /// with a strong authentication system and hence implicitely implies the 'no-auth' option
    /// as well.
    #[clap(short, long)]
    pub passcode: Option<String>,
    /// Local redo log file will be deleted when the file system is unmounted, remotely stored data on
    /// any distributed commit log will be persisted. Effectively this setting only uses the local disk
    /// as a cache of the redo-log while it's being used.
    #[clap(long)]
    pub temp: bool,
    /// UID of the user that this file system will be mounted as
    #[clap(short, long)]
    pub uid: Option<u32>,
    /// GID of the group that this file system will be mounted as
    #[clap(short, long)]
    pub gid: Option<u32>,
    /// Allow the root user to have access to this file system
    #[clap(long)]
    pub allow_root: bool,
    /// Allow other users on the machine to have access to this file system
    #[clap(long)]
    pub allow_other: bool,
    /// Mount the file system in readonly mode (`ro` mount option), default is disable.
    #[clap(short, long)]
    pub read_only: bool,
    /// Enable write back cache for buffered writes, default is disable.
    #[clap(short, long)]
    pub write_back: bool,
    /// Allow fuse filesystem mount on a non-empty directory, default is not allowed.
    #[clap(long)]
    pub non_empty: bool,
    /// For files and directories that the authenticated user owns, translate the UID and GID to the local machine ids instead of the global ones.
    #[clap(short, long)]
    pub impersonate_uid: bool, 
    /// Configure the log file for <raw>, <barebone>, <speed>, <compatibility>, <balanced> or <security>
    #[clap(long, default_value = "speed")]
    pub configured_for: ate::conf::ConfiguredFor,
    /// Format of the metadata in the log file as <bincode>, <json> or <mpack>
    #[clap(long, default_value = "bincode")]
    pub meta_format: ate::spec::SerializationFormat,
    /// Format of the data in the log file as <bincode>, <json> or <mpack>
    #[clap(long, default_value = "bincode")]
    pub data_format: ate::spec::SerializationFormat,
    /// Forces the compaction of the local redo-log before it streams in the latest values
    #[clap(long)]
    pub compact_now: bool,
    /// Mode that the compaction will run under (valid modes are 'never', 'modified', 'timer', 'factor', 'size', 'factor-or-timer', 'size-or-timer')
    #[clap(long, default_value = "factor-or-timer")]
    pub compact_mode: CompactMode,
    /// Time in seconds between compactions of the log file (default: 1 hour) - this argument is ignored if you select a compact_mode that has no timer
    #[clap(long, default_value = "3600")]
    pub compact_timer: u64,
    /// Factor growth in the log file which will trigger compaction - this argument is ignored if you select a compact_mode that has no growth trigger
    #[clap(long, default_value = "0.4")]
    pub compact_threshold_factor: f32,
    /// Size of growth in bytes in the log file which will trigger compaction (default: 100MB) - this argument is ignored if you select a compact_mode that has no growth trigger
    #[clap(long, default_value = "104857600")]
    pub compact_threshold_size: u64,
}