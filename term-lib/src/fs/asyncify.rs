use std::ops::Deref;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use wasmer_vfs::*;

use crate::api::*;

#[derive(Debug, Clone)]
pub struct AsyncifyFileSystem {
    system: System,
    fs: Arc<dyn FileSystem>,
}

impl AsyncifyFileSystem {
    pub fn new(fs: impl FileSystem) -> AsyncifyFileSystem {
        AsyncifyFileSystem {
            system: System::default(),
            fs: Arc::new(fs),
        }
    }

    async fn asyncify<T>(&self, funct: impl FnOnce(&dyn FileSystem) -> T + Send + 'static) -> T
    where
        T: Send,
    {
        let fs = self.fs.clone();
        self.system
            .spawn_dedicated(move || async move { funct(fs.deref()) })
            .join()
            .await
            .unwrap()
    }

    pub async fn read_dir(&self, path: &Path) -> Result<ReadDir> {
        let path = path.to_owned();
        self.asyncify(move |fs| fs.read_dir(path.as_path())).await
    }

    pub async fn create_dir(&self, path: &Path) -> Result<()> {
        let path = path.to_owned();
        self.asyncify(move |fs| fs.create_dir(path.as_path())).await
    }

    pub async fn remove_dir(&self, path: &Path) -> Result<()> {
        let path = path.to_owned();
        self.asyncify(move |fs| fs.remove_dir(path.as_path())).await
    }

    pub async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        let from = from.to_owned();
        let to = to.to_owned();
        self.asyncify(move |fs| fs.rename(from.as_path(), to.as_path()))
            .await
    }

    pub async fn metadata(&self, path: &Path) -> Result<Metadata> {
        let path = path.to_owned();
        self.asyncify(move |fs| fs.metadata(path.as_path())).await
    }

    pub async fn symlink_metadata(&self, path: &Path) -> Result<Metadata> {
        let path = path.to_owned();
        self.asyncify(move |fs| fs.symlink_metadata(path.as_path()))
            .await
    }

    pub async fn remove_file(&self, path: &Path) -> Result<()> {
        let path = path.to_owned();
        self.asyncify(move |fs| fs.remove_file(path.as_path()))
            .await
    }

    pub async fn new_open_options(&self) -> AsyncifyOpenOptions {
        let opener = Box::new(AsyncifyFileOpener {
            system: self.system,
            parent: self.clone(),
        });
        AsyncifyOpenOptions::new(opener)
    }
}

#[derive(Clone)]
struct AsyncifyOpenOptionsConf {
    read: bool,
    write: bool,
    create_new: bool,
    create: bool,
    append: bool,
    truncate: bool,
}

pub struct AsyncifyOpenOptions {
    opener: Box<AsyncifyFileOpener>,
    conf: AsyncifyOpenOptionsConf,
}

impl AsyncifyOpenOptions {
    pub fn new(opener: Box<AsyncifyFileOpener>) -> Self {
        Self {
            opener,
            conf: AsyncifyOpenOptionsConf {
                read: false,
                write: false,
                create_new: false,
                create: false,
                append: false,
                truncate: false,
            },
        }
    }

    pub fn set_options(&mut self, options: OpenOptionsConfig) -> &mut Self {
        self.conf.read = options.read();
        self.conf.write = options.write();
        self.conf.create_new = options.create_new();
        self.conf.create = options.create();
        self.conf.append = options.append();
        self.conf.truncate = options.truncate();
        self
    }

    pub fn read(&mut self, read: bool) -> &mut Self {
        self.conf.read = read;
        self
    }

    pub fn write(&mut self, write: bool) -> &mut Self {
        self.conf.write = write;
        self
    }

    pub fn append(&mut self, append: bool) -> &mut Self {
        self.conf.append = append;
        self
    }

    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.conf.truncate = truncate;
        self
    }

    pub fn create(&mut self, create: bool) -> &mut Self {
        self.conf.create = create;
        self
    }

    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.conf.create_new = create_new;
        self
    }

    pub async fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<AsyncifyVirtualFile> {
        self.opener.open(path.as_ref(), self.conf.clone()).await
    }
}

pub struct AsyncifyFileOpener {
    system: System,
    parent: AsyncifyFileSystem,
}

impl AsyncifyFileOpener {
    async fn open(
        &mut self,
        path: &Path,
        conf: AsyncifyOpenOptionsConf,
    ) -> Result<AsyncifyVirtualFile> {
        let path = path.to_owned();
        let parent = self.parent.clone();

        self.system
            .spawn_dedicated(move || async move {
                let mut options = parent.fs.new_open_options();
                options.read(conf.read);
                options.write(conf.write);
                options.create_new(conf.create_new);
                options.create(conf.create);
                options.append(conf.append);
                options.truncate(conf.truncate);

                let file = options.open(path)?;

                Ok(AsyncifyVirtualFile {
                    system: System::default(),
                    file: Arc::new(Mutex::new(file)),
                })
            })
            .join()
            .await
            .unwrap()
    }
}

pub struct AsyncifyVirtualFile {
    system: System,
    file: Arc<Mutex<Box<dyn VirtualFile + Sync>>>,
}

impl AsyncifyVirtualFile {
    async fn asyncify<T>(
        &self,
        funct: impl FnOnce(&mut (dyn VirtualFile + Sync)) -> T + Send + 'static,
    ) -> T
    where
        T: Send,
    {
        let file = self.file.clone();
        self.system
            .spawn_dedicated(move || async move {
                let mut file = file.lock().unwrap();
                let file = file.deref_mut().deref_mut();
                funct(file)
            })
            .join()
            .await
            .unwrap()
    }

    /// the last time the file was accessed in nanoseconds as a UNIX timestamp
    pub async fn last_accessed(&self) -> u64 {
        self.asyncify(move |file| file.last_accessed()).await
    }

    /// the last time the file was modified in nanoseconds as a UNIX timestamp
    pub async fn last_modified(&self) -> u64 {
        self.asyncify(move |file| file.last_modified()).await
    }

    /// the time at which the file was created in nanoseconds as a UNIX timestamp
    pub async fn created_time(&self) -> u64 {
        self.asyncify(move |file| file.created_time()).await
    }

    /// the size of the file in bytes
    pub async fn size(&self) -> u64 {
        self.asyncify(move |file| file.size()).await
    }

    /// Change the size of the file, if the `new_size` is greater than the current size
    /// the extra bytes will be allocated and zeroed
    pub async fn set_len(&mut self, new_size: u64) -> Result<()> {
        self.asyncify(move |file| file.set_len(new_size)).await
    }

    /// Request deletion of the file
    pub async fn unlink(&mut self) -> Result<()> {
        self.asyncify(move |file| file.unlink()).await
    }

    /// Store file contents and metadata to disk
    /// Default implementation returns `Ok(())`.  You should implement this method if you care
    /// about flushing your cache to permanent storage
    pub async fn sync_to_disk(&self) -> Result<()> {
        self.asyncify(move |file| file.sync_to_disk()).await
    }

    /// Returns the number of bytes available.  This function must not block
    pub async fn bytes_available(&self) -> Result<usize> {
        self.asyncify(move |file| file.bytes_available()).await
    }

    /// Returns the number of bytes available.  This function must not block
    /// Defaults to `None` which means the number of bytes is unknown
    pub async fn bytes_available_read(&self) -> Result<Option<usize>> {
        self.asyncify(move |file| file.bytes_available_read()).await
    }

    /// Returns the number of bytes available.  This function must not block
    /// Defaults to `None` which means the number of bytes is unknown
    pub async fn bytes_available_write(&self) -> Result<Option<usize>> {
        self.asyncify(move |file| file.bytes_available_write())
            .await
    }

    // Indicates if the file has been open or close. This function must not block
    // Defaults to a status of constantly open
    pub async fn is_open(&self) -> bool {
        self.asyncify(move |file| file.is_open()).await
    }
}

impl AsyncifyVirtualFile {
    pub async fn read(&self, max: usize) -> Result<Vec<u8>> {
        self.asyncify(move |file| {
            let mut buf = Vec::with_capacity(max);
            unsafe {
                buf.set_len(max);
                let read = file.read(&mut buf[..])?;
                buf.set_len(read);
            }
            Ok(buf)
        })
        .await
    }

    pub async fn write_all(&mut self, buf: Vec<u8>) -> Result<()> {
        self.asyncify(move |file| {
            file.write_all(&buf[..])?;
            Ok(())
        })
        .await
    }

    pub async fn read_to_end(&mut self) -> Result<Vec<u8>> {
        self.asyncify(move |file| {
            let mut ret = Vec::new();
            file.read_to_end(&mut ret)?;
            Ok(ret)
        })
        .await
    }

    pub async fn read_to_string(&mut self) -> Result<String> {
        self.asyncify(move |file| {
            let mut ret = String::new();
            file.read_to_string(&mut ret)?;
            Ok(ret)
        })
        .await
    }
}
