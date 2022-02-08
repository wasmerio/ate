use ate::mesh::Registry;
use ate::session::AteSessionType;
use ate::session::AteSessionUser;
use ate::transaction::TransactionScope;
use ate_files::prelude::*;
use ate::prelude::AteErrorKind;
use std::sync::Arc;
use tokio::sync::Mutex;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Clone)]
pub struct NativeFiles {
    registry: Arc<Registry>,
    db_url: url::Url,
    native_files_name: String,
    native_files: Arc<Mutex<Option<Arc<FileAccessor>>>>,
}

impl NativeFiles {
    pub fn new(registry: Arc<Registry>, db_url: url::Url, native_files: String) -> Self {
        Self {
            registry,
            db_url,
            native_files_name: native_files,
            native_files: Arc::new(Mutex::new(None))
        }    
    }

    pub async fn get(&self) -> Result<Arc<FileAccessor>, FileSystemError> {
        // Lock and fast path
        let mut guard = self.native_files.lock().await;
        if guard.is_some() {
            return Ok(guard.as_ref().unwrap().clone());
        }

        // Connect to the file system that holds all the binaries that
        // we will present natively to the consumers
        // Note: These are the same files presenting to the web-site version of the terminal
        let native_files_key = ate::prelude::ChainKey::from(self.native_files_name.clone());
        let native_files = self.registry.open(&self.db_url, &native_files_key).await
            .map_err(|err| FileSystemErrorKind::AteError(AteErrorKind::ChainCreationError(err.0)))?;
        let native_files = Arc::new(
            FileAccessor::new(
                native_files.as_arc(),
                None,
                AteSessionType::User(AteSessionUser::default()),
                TransactionScope::Local,
                TransactionScope::Local,
                true,
                false,
            )
            .await,
        );

        // Attempt to read the root from the native file system which will make sure that its
        // all nicely running
        native_files
            .search(&RequestContext { uid: 0, gid: 0 }, "/")
            .await?;

        // Set the object and return
        guard.replace(native_files.clone());
        Ok(native_files)
    }
}