//! Handle-relative filesystem adapter. Call blocking operations on a worker thread.
//! Only `Storage::open` accepts a native host path; all later calls use VirtualPath.
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::fs::{Dir, Metadata, OpenOptions};
use splitshare_core::{ConflictPolicy, EntryKind, FileEntry, StorageError, VirtualPath};
use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

mod native;

fn io_error(error: io::Error) -> StorageError {
    // Log only the error kind: OS messages may contain native paths.
    tracing::debug!(kind = ?error.kind(), "Storage operation failed");
    match error.kind() {
        io::ErrorKind::NotFound => StorageError::NotFound,
        io::ErrorKind::AlreadyExists => StorageError::Conflict,
        io::ErrorKind::PermissionDenied => StorageError::PermissionDenied,
        io::ErrorKind::NotADirectory => StorageError::NotDirectory,
        io::ErrorKind::DirectoryNotEmpty => StorageError::DirectoryNotEmpty,
        io::ErrorKind::Unsupported => StorageError::UnsupportedOperation,
        _ => StorageError::Io,
    }
}

fn supported(metadata: &Metadata) -> Result<(), StorageError> {
    #[cfg(windows)]
    {
        use cap_std::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(StorageError::UnsupportedEntry);
        }
    }
    if metadata.is_symlink() || !(metadata.is_dir() || metadata.is_file()) {
        return Err(StorageError::UnsupportedEntry);
    }
    Ok(())
}

struct Inner {
    root: Dir,
    mutations: Mutex<()>,
}
#[derive(Clone)]
pub struct Storage {
    inner: Arc<Inner>,
}

impl Storage {
    /// Native host boundary. The selected directory itself must not be a link.
    pub fn open(root: &Path) -> Result<Self, StorageError> {
        let metadata = std::fs::symlink_metadata(root).map_err(io_error)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(StorageError::UnsupportedEntry);
            }
        }
        if metadata.is_symlink() {
            return Err(StorageError::UnsupportedEntry);
        }
        if !metadata.is_dir() {
            return Err(StorageError::NotDirectory);
        }
        // Ambient authority is used only at host selection. Open the last component
        // with no-follow as well, closing a link-swap race after the metadata check.
        let absolute = std::path::absolute(root).map_err(io_error)?;
        let root = if let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name()) {
            Dir::open_ambient_dir(parent, cap_std::ambient_authority())
                .map_err(io_error)?
                .open_dir_nofollow(name)
                .map_err(io_error)?
        } else {
            Dir::open_ambient_dir(&absolute, cap_std::ambient_authority()).map_err(io_error)?
        };
        supported(&root.dir_metadata().map_err(io_error)?)?;
        Ok(Self {
            inner: Arc::new(Inner {
                root,
                mutations: Mutex::new(()),
            }),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, ()>, StorageError> {
        self.inner.mutations.lock().map_err(|_| StorageError::Io)
    }

    fn directory(&self, path: &VirtualPath) -> Result<Dir, StorageError> {
        let mut directory = self.inner.root.try_clone().map_err(io_error)?;
        for component in path.components() {
            supported(&directory.symlink_metadata(component).map_err(io_error)?)?;
            directory = directory.open_dir_nofollow(component).map_err(io_error)?;
            supported(&directory.dir_metadata().map_err(io_error)?)?;
        }
        Ok(directory)
    }

    fn parent(&self, path: &VirtualPath) -> Result<(Dir, String), StorageError> {
        if path.is_root() {
            return Err(StorageError::RootProtected);
        }
        let (parent, name) = path
            .as_str()
            .rsplit_once('/')
            .ok_or(StorageError::InvalidPath)?;
        let parent = VirtualPath::try_from(if parent.is_empty() { "/" } else { parent })?;
        Ok((self.directory(&parent)?, name.to_owned()))
    }

    pub fn metadata(&self, path: &VirtualPath) -> Result<FileEntry, StorageError> {
        let metadata = if path.is_root() {
            self.inner.root.dir_metadata().map_err(io_error)?
        } else {
            let (dir, name) = self.parent(path)?;
            dir.symlink_metadata(name).map_err(io_error)?
        };
        entry(path.clone(), &metadata)
    }

    /// Hidden entries, links, reserved upload files and unsupported names are omitted.
    pub fn list(&self, path: &VirtualPath) -> Result<Vec<FileEntry>, StorageError> {
        let dir = self.directory(path)?;
        let mut result = Vec::new();
        for item in dir.entries().map_err(io_error)? {
            let item = item.map_err(io_error)?;
            let Some(name) = item.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let Ok(path) = path.join(&name) else {
                continue;
            };
            let metadata = match dir.symlink_metadata(&name) {
                Ok(m) => m,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(io_error(e)),
            };
            if supported(&metadata).is_err() {
                continue;
            }
            #[cfg(windows)]
            {
                use cap_std::fs::MetadataExt;
                if metadata.file_attributes() & 6 != 0 {
                    continue;
                }
            }
            result.push(entry(path, &metadata)?);
        }
        result.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(result)
    }

    pub fn mkdir(&self, path: &VirtualPath) -> Result<(), StorageError> {
        let _guard = self.lock()?;
        let (dir, name) = self.parent(path)?;
        dir.create_dir(name).map_err(io_error)
    }

    /// Rename never replaces an existing destination, including during races.
    pub fn rename(&self, from: &VirtualPath, to: &VirtualPath) -> Result<(), StorageError> {
        let _guard = self.lock()?;
        let (src, old) = self.parent(from)?;
        let (dst, new) = self.parent(to)?;
        supported(&src.symlink_metadata(&old).map_err(io_error)?)?;
        native::rename_noreplace(&src, &old, &dst, &new).map_err(io_error)
    }

    /// Deletes files or empty directories. Recursive deletion is deliberately explicit later.
    pub fn delete(&self, path: &VirtualPath) -> Result<(), StorageError> {
        let _guard = self.lock()?;
        let (dir, name) = self.parent(path)?;
        let metadata = dir.symlink_metadata(&name).map_err(io_error)?;
        supported(&metadata)?;
        if metadata.is_dir() {
            dir.remove_dir(name)
        } else {
            dir.remove_file(name)
        }
        .map_err(io_error)
    }

    pub fn read(&self, path: &VirtualPath) -> Result<ReadHandle, StorageError> {
        let (dir, name) = self.parent(path)?;
        let metadata = dir.symlink_metadata(&name).map_err(io_error)?;
        supported(&metadata)?;
        if !metadata.is_file() {
            return Err(StorageError::NotFile);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No).nonblock(true);
        let file = dir.open_with(name, &options).map_err(io_error)?;
        let metadata = file.metadata().map_err(io_error)?;
        supported(&metadata)?;
        if !metadata.is_file() {
            return Err(StorageError::NotFile);
        }
        Ok(ReadHandle {
            file: file.into_std(),
            len: metadata.len(),
        })
    }

    pub fn begin_upload(
        &self,
        destination: &VirtualPath,
        expected: Option<u64>,
    ) -> Result<Upload, StorageError> {
        let _guard = self.lock()?;
        let (dir, _) = self.parent(destination)?;
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| StorageError::Io)?;
        let id: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let temporary = format!(".splitshare-upload-{id}.part");
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        let file = dir
            .open_with(&temporary, &options)
            .map_err(io_error)?
            .into_std();
        Ok(Upload {
            storage: self.clone(),
            dir,
            temporary,
            file: Some(file),
            destination: destination.clone(),
            expected,
            written: 0,
            failed: false,
            published: false,
        })
    }
}

fn entry(path: VirtualPath, metadata: &Metadata) -> Result<FileEntry, StorageError> {
    supported(metadata)?;
    Ok(FileEntry {
        name: path.components().last().unwrap_or("/").to_owned(),
        path,
        kind: if metadata.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::File
        },
        size: metadata.is_file().then_some(metadata.len()),
        modified_unix_seconds: metadata
            .modified()
            .ok()
            .and_then(|time| time.into_std().duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs()),
    })
}

pub struct ReadHandle {
    file: std::fs::File,
    pub len: u64,
}
impl ReadHandle {
    pub fn into_file(self) -> std::fs::File {
        self.file
    }
}
impl Read for ReadHandle {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.file.read(buffer)
    }
}
impl Seek for ReadHandle {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.file.seek(position)
    }
}

/// A bounded-chunk writer. Only `publish` makes data visible; Drop removes partial data.
pub struct Upload {
    storage: Storage,
    dir: Dir,
    temporary: String,
    file: Option<std::fs::File>,
    destination: VirtualPath,
    expected: Option<u64>,
    written: u64,
    failed: bool,
    published: bool,
}
impl Upload {
    pub fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), StorageError> {
        if self.failed {
            return Err(StorageError::UploadFailed);
        }
        if chunk.len() > 64 * 1024 {
            self.failed = true;
            return Err(StorageError::UploadFailed);
        }
        let length = self
            .written
            .checked_add(chunk.len() as u64)
            .ok_or(StorageError::LengthMismatch)?;
        if self.expected.is_some_and(|expected| length > expected) {
            self.failed = true;
            return Err(StorageError::LengthMismatch);
        }
        if let Err(error) = self
            .file
            .as_mut()
            .ok_or(StorageError::UploadFailed)?
            .write_all(chunk)
        {
            self.failed = true;
            return Err(io_error(error));
        }
        self.written = length;
        Ok(())
    }
    pub fn bytes_written(&self) -> u64 {
        self.written
    }
    pub fn cancel(self) -> Result<(), StorageError> {
        self.cleanup()
    }
    fn cleanup(mut self) -> Result<(), StorageError> {
        self.file.take();
        self.dir.remove_file(&self.temporary).map_err(io_error)?;
        self.published = true;
        Ok(())
    }
    pub fn publish(mut self, policy: ConflictPolicy) -> Result<VirtualPath, StorageError> {
        if self.failed {
            return Err(StorageError::UploadFailed);
        }
        if self.expected.is_some_and(|length| length != self.written) {
            return Err(StorageError::LengthMismatch);
        }
        self.file
            .as_mut()
            .ok_or(StorageError::UploadFailed)?
            .flush()
            .map_err(io_error)?;
        self.file
            .as_ref()
            .ok_or(StorageError::UploadFailed)?
            .sync_all()
            .map_err(io_error)?;
        self.file.take();
        let _guard = self.storage.lock()?;
        // Resolve again: a removed or renamed destination parent must not publish at its old path.
        let (target_dir, name) = self.storage.parent(&self.destination)?;
        for attempt in 0..=1000 {
            let candidate = if attempt == 0 {
                name.clone()
            } else {
                format!("{name} ({attempt})")
            };
            let parent = self.destination.as_str().rsplit_once('/').unwrap().0;
            let path = VirtualPath::try_from(format!("{parent}/{candidate}"))?;
            if policy == ConflictPolicy::Replace {
                match target_dir.symlink_metadata(&candidate) {
                    Ok(metadata) => {
                        supported(&metadata)?;
                        if !metadata.is_file() {
                            return Err(StorageError::NotFile);
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => (),
                    Err(error) => return Err(io_error(error)),
                }
                native::rename_replace(&self.dir, &self.temporary, &target_dir, &candidate)
                    .map_err(io_error)?;
            } else {
                match native::rename_noreplace(&self.dir, &self.temporary, &target_dir, &candidate)
                {
                    Ok(()) => (),
                    Err(error)
                        if error.kind() == io::ErrorKind::AlreadyExists
                            && policy == ConflictPolicy::AutoRename =>
                    {
                        continue;
                    }
                    Err(error) => return Err(io_error(error)),
                }
            }
            self.published = true;
            tracing::debug!("Upload published");
            return Ok(path);
        }
        Err(StorageError::Conflict)
    }
}
impl Drop for Upload {
    fn drop(&mut self) {
        self.file.take();
        if !self.published
            && let Err(error) = self.dir.remove_file(&self.temporary)
            && error.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(kind = ?error.kind(), "Partial upload cleanup failed");
        }
    }
}
