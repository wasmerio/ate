use ate::error::AteError;

use crate::opt::*;

pub async fn main_opts_logout(
    _opts_logout: OptsLogout,
    token_path: String,
) -> Result<(), AteError> {
    // Convert the token path to a real path
    let token_path = shellexpand::tilde(&token_path).to_string();

    // Remove any old paths
    if let Ok(old) = std::fs::canonicalize(token_path.clone()) {
        let _ = std::fs::remove_file(old);
    }
    let _ = std::fs::remove_file(token_path.clone());

    Ok(())
}
