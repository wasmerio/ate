use ate::error::AteError;
#[cfg(target_os = "wasi")]
use wasm_bus_process::prelude::*;

use crate::opt::*;

pub async fn main_opts_logout(
    _opts_logout: OptsLogout,
    token_path: String,
) -> Result<(), AteError> {
    // Convert the token path to a real path
    let token_network_path = format!("{}.network", token_path);
    let token_network_path = shellexpand::tilde(&token_network_path).to_string();
    let token_path = shellexpand::tilde(&token_path).to_string();
    
    // Remove any old paths
    if let Ok(old) = std::fs::canonicalize(token_network_path.clone()) {
        let _ = std::fs::remove_file(old);
    }
    let _ = std::fs::remove_file(token_network_path.clone());
    if let Ok(old) = std::fs::canonicalize(token_path.clone()) {
        let _ = std::fs::remove_file(old);
    }
    let _ = std::fs::remove_file(token_path.clone());

    // If we are in WASM mode and there is a logout script then run it
    #[cfg(target_os = "wasi")]
    if std::path::Path::new("/usr/etc/logout.sh").exists() == true {
        Command::new(format!("source").as_str())
            .args(&["/usr/etc/logout.sh"])
            .execute()
            .await?;
    }
    #[cfg(target_os = "wasi")]
    if std::path::Path::new("/etc/logout.sh").exists() == true {
        Command::new(format!("source").as_str())
            .args(&["/etc/logout.sh"])
            .execute()
            .await?;
    }

    Ok(())
}
