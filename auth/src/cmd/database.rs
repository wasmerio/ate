#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use ate::prelude::*;
use std::sync::Arc;
use url::Url;
use std::io::stdout;
use std::io::Write;
use ate::utils::LoadProgress;

use crate::prelude::*;
use crate::helper::*;
use crate::error::*;
use crate::request::*;
use crate::opt::*;
use crate::cmd::*;

pub async fn main_opts_db(opts_db: OptsDatabase, token: Option<String>, token_path: Option<String>, auth: url::Url, hint_group: &str) -> Result<(), AteError>
{
    let db_name = match &opts_db.action {
        DatabaseAction::Truncate(action) => action.name.clone(),
        DatabaseAction::Details(action) => action.name.clone(),
    };

    let group_name = match db_name.split("/").map(|a| a.to_string()).next() {
        Some(a) => a,
        None => {
            eprintln!("The database name is invalid");
            std::process::exit(1);
        }
    };

    // Build the conf and registry
    let conf = ConfAte::default();
    let session = main_session_group(token.clone(), token_path.clone(), group_name.clone(), true, None, Some(auth.clone()), hint_group).await?;
    let registry = ate::mesh::Registry::new(&conf).await
        .temporal(true);

    // Create a progress bar loader
    let progress_local = LoadProgress::default();
    let progress_remote = LoadProgress::default();
    
    // Load the chain
    let guard = registry.open_ext(&opts_db.remote, &ChainKey::from(db_name.clone()), progress_local, progress_remote).await?;
    let db = guard.as_arc();

    match opts_db.action {
        DatabaseAction::Details(_action) => {
            let guard = db.metrics().lock();
            println!("Database Chain Details");
            println!("======================");
            println!("Remote: {}", opts_db.remote);
            println!("Group Name: {}", group_name);
            println!("DB Name: {}", db_name);
            println!("Size: {}", guard.chain_size);
        },
        DatabaseAction::Truncate(_action) => {
            print!("Deleting all events");
            let dio = db.dio_full(&session).await;
            let mut ids = dio.dio.all_keys().await;
            while ids.is_empty() == false {
                print!(".");
                for _ in 0..100 {
                    let id = match ids.pop() {
                        Some(a) => a,
                        None => break
                    };
                    let _ = dio.delete(&id).await;
                }
                dio.commit().await?;
            }
            println!("Done");
        }
    }
    Ok(())
}