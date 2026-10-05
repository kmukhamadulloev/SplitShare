use splitshare_core::{ConflictPolicy as Policy, StorageError as Error, VirtualPath};
use splitshare_storage::Storage;
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    sync::{Arc, Barrier},
};
fn path(value: &str) -> VirtualPath {
    VirtualPath::try_from(value).unwrap()
}
fn setup() -> (tempfile::TempDir, Storage) {
    let root = tempfile::tempdir().unwrap();
    let storage = Storage::open(root.path()).unwrap();
    (root, storage)
}
fn upload(
    storage: &Storage,
    name: &str,
    bytes: &[u8],
    policy: Policy,
) -> Result<VirtualPath, Error> {
    let mut upload = storage.begin_upload(&path(name), Some(bytes.len() as u64))?;
    upload.write_chunk(bytes)?;
    upload.publish(policy)
}
#[test]
fn directory_file_lifecycle_and_seek() {
    let (_root, storage) = setup();
    storage.mkdir(&path("/資料")).unwrap();
    upload(&storage, "/資料/a.txt", b"abcdef", Policy::Reject).unwrap();
    assert_eq!(
        storage.metadata(&path("/資料/a.txt")).unwrap().size,
        Some(6)
    );
    let mut read = storage.read(&path("/資料/a.txt")).unwrap();
    read.seek(SeekFrom::Start(2)).unwrap();
    let mut buffer = [0; 3];
    read.read_exact(&mut buffer).unwrap();
    assert_eq!(&buffer, b"cde");
    drop(read);
    storage
        .rename(&path("/資料/a.txt"), &path("/資料/b.txt"))
        .unwrap();
    assert_eq!(storage.list(&path("/資料")).unwrap()[0].name, "b.txt");
    assert_eq!(
        storage.delete(&path("/資料")),
        Err(Error::DirectoryNotEmpty)
    );
    storage.delete(&path("/資料/b.txt")).unwrap();
    storage.rename(&path("/資料"), &path("/renamed")).unwrap();
    storage.delete(&path("/renamed")).unwrap();
    assert!(storage.list(&path("/")).unwrap().is_empty());
}
#[test]
fn root_boundaries_and_redaction() {
    let (root, storage) = setup();
    for result in [
        storage.delete(&path("/")),
        storage.rename(&path("/"), &path("/x")),
        storage.rename(&path("/x"), &path("/")),
        storage.mkdir(&path("/")),
    ] {
        assert_eq!(result, Err(Error::RootProtected));
    }
    assert!(storage.begin_upload(&path("/"), None).is_err());
    fs::write(root.path().join("visible"), b"x").unwrap();
    let json = serde_json::to_string(&storage.list(&path("/")).unwrap()).unwrap();
    assert!(!json.contains(root.path().to_str().unwrap()));
    let error = storage.metadata(&path("/missing")).unwrap_err();
    assert_eq!(error, Error::NotFound);
    assert!(!format!("{error:?} {error}").contains(root.path().to_str().unwrap()));
}
#[test]
fn partial_uploads_are_hidden_unaddressable_and_cleaned() {
    let (root, storage) = setup();
    {
        let mut writer = storage.begin_upload(&path("/complete"), Some(5)).unwrap();
        writer.write_chunk(b"abc").unwrap();
        assert!(storage.list(&path("/")).unwrap().is_empty());
        assert_eq!(
            storage.metadata(&path("/complete")).unwrap_err(),
            Error::NotFound
        );
        let temporary = fs::read_dir(root.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .file_name();
        assert!(VirtualPath::try_from(format!("/{}", temporary.to_str().unwrap())).is_err());
    }
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    let mut writer = storage.begin_upload(&path("/complete"), Some(5)).unwrap();
    writer.write_chunk(b"abc").unwrap();
    assert_eq!(writer.publish(Policy::Reject), Err(Error::LengthMismatch));
    let mut writer = storage.begin_upload(&path("/complete"), Some(1)).unwrap();
    assert_eq!(writer.write_chunk(b"too much"), Err(Error::LengthMismatch));
    assert_eq!(writer.publish(Policy::Reject), Err(Error::UploadFailed));
    storage
        .begin_upload(&path("/complete"), None)
        .unwrap()
        .cancel()
        .unwrap();
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}
#[test]
fn conflicts_replace_and_auto_rename() {
    let (root, storage) = setup();
    upload(&storage, "/file", b"old", Policy::Reject).unwrap();
    for policy in [Policy::Ask, Policy::Reject] {
        assert_eq!(
            upload(&storage, "/file", b"new", policy),
            Err(Error::Conflict)
        );
        assert_eq!(fs::read(root.path().join("file")).unwrap(), b"old");
    }
    assert_eq!(
        upload(&storage, "/file", b"copy", Policy::AutoRename).unwrap(),
        path("/file (1)")
    );
    upload(&storage, "/file", b"new", Policy::Replace).unwrap();
    assert_eq!(fs::read(root.path().join("file")).unwrap(), b"new");
    assert_eq!(
        storage.rename(&path("/file"), &path("/file (1)")),
        Err(Error::Conflict)
    );
    storage.mkdir(&path("/folder")).unwrap();
    assert_eq!(
        upload(&storage, "/folder", b"bad", Policy::Replace),
        Err(Error::NotFile)
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
}
#[test]
fn simultaneous_publish_has_exactly_one_winner() {
    let (root, storage) = setup();
    let barrier = Arc::new(Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|id| {
            // Independent instances ensure OS no-clobber works beyond the in-process lock.
            let storage = Storage::open(root.path()).unwrap();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut writer = storage.begin_upload(&path("/winner"), Some(1)).unwrap();
                writer.write_chunk(&[id]).unwrap();
                barrier.wait();
                writer.publish(Policy::Reject)
            })
        })
        .collect();
    let results: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == Err(Error::Conflict))
            .count(),
        7
    );
    assert_eq!(storage.list(&path("/")).unwrap().len(), 1);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}
#[test]
fn stream_large_file_in_fixed_chunks() {
    let (_root, storage) = setup();
    let chunk = [42; 65536];
    let mut writer = storage
        .begin_upload(&path("/large"), Some(16 * 1024 * 1024))
        .unwrap();
    for _ in 0..256 {
        writer.write_chunk(&chunk).unwrap();
    }
    assert_eq!(writer.bytes_written(), 16 * 1024 * 1024);
    writer.publish(Policy::Reject).unwrap();
    let mut read = storage.read(&path("/large")).unwrap();
    assert_eq!(read.len, 16 * 1024 * 1024);
    let mut buffer = [0; 65536];
    for _ in 0..256 {
        read.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer, chunk);
    }
    assert_eq!(read.read(&mut buffer).unwrap(), 0);
}

#[cfg(unix)]
fn symlink_dir(target: &std::path::Path, link: &std::path::Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}
#[cfg(windows)]
fn symlink_dir(target: &std::path::Path, link: &std::path::Path) {
    std::os::windows::fs::symlink_dir(target, link)
        .expect("Windows symlink tests require Developer Mode or elevated runner");
}

#[test]
fn links_cannot_escape_or_be_mutated() {
    let (root, storage) = setup();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), b"private").unwrap();
    symlink_dir(outside.path(), &root.path().join("escape"));
    assert!(storage.list(&path("/escape")).is_err());
    assert!(storage.read(&path("/escape/secret")).is_err());
    assert!(storage.metadata(&path("/escape/secret")).is_err());
    assert!(storage.mkdir(&path("/escape/new")).is_err());
    assert!(storage.begin_upload(&path("/escape/new"), None).is_err());
    assert!(storage.rename(&path("/escape"), &path("/moved")).is_err());
    assert!(storage.delete(&path("/escape")).is_err());
    assert!(storage.list(&path("/")).unwrap().is_empty());
    assert!(Storage::open(&root.path().join("escape")).is_err());
    assert_eq!(fs::read(outside.path().join("secret")).unwrap(), b"private");
}

#[test]
fn swapped_upload_parent_cannot_publish_outside_root() {
    let (root, storage) = setup();
    let outside = tempfile::tempdir().unwrap();
    storage.mkdir(&path("/destination")).unwrap();
    let mut writer = storage
        .begin_upload(&path("/destination/file"), Some(1))
        .unwrap();
    writer.write_chunk(b"x").unwrap();
    let swap = fs::rename(root.path().join("destination"), root.path().join("old"));
    #[cfg(windows)]
    if let Err(error) = &swap {
        // Windows may prevent renaming a directory containing an open upload.
        // Only this precise sharing violation is a valid blocked-swap outcome;
        // other failures must still fail the regression.
        assert_eq!(error.raw_os_error(), Some(32), "{error:?}");
        assert!(!root.path().join("old").exists());
        assert!(!root.path().join("destination/file").exists());
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
        drop(writer);
        assert_eq!(
            fs::read_dir(root.path().join("destination"))
                .unwrap()
                .count(),
            0
        );

        // Once handles are released, perform the actual swap and verify that a
        // fresh upload rejects the replacement link rather than writing outside.
        fs::rename(root.path().join("destination"), root.path().join("old")).unwrap();
        symlink_dir(outside.path(), &root.path().join("destination"));
        assert!(
            storage
                .begin_upload(&path("/destination/file"), Some(1))
                .is_err()
        );
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
        assert_eq!(fs::read_dir(root.path().join("old")).unwrap().count(), 0);
        return;
    }
    swap.unwrap();
    symlink_dir(outside.path(), &root.path().join("destination"));
    assert!(writer.publish(Policy::Reject).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    assert_eq!(fs::read_dir(root.path().join("old")).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn final_symlinks_and_special_files_are_denied() {
    let (root, storage) = setup();
    fs::write(root.path().join("real"), b"safe").unwrap();
    std::os::unix::fs::symlink("real", root.path().join("link")).unwrap();
    assert!(storage.read(&path("/link")).is_err());
    assert_eq!(
        upload(&storage, "/link", b"bad", Policy::Replace),
        Err(Error::UnsupportedEntry)
    );
    let _socket = std::os::unix::net::UnixListener::bind(root.path().join("socket")).unwrap();
    assert!(storage.read(&path("/socket")).is_err());
    assert_eq!(storage.list(&path("/")).unwrap().len(), 1);
    assert_eq!(fs::read(root.path().join("real")).unwrap(), b"safe");
}

#[cfg(unix)]
#[test]
fn concurrent_symlink_swap_never_reads_external_content() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let (root, storage) = setup();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), b"private").unwrap();
    fs::create_dir(root.path().join("directory")).unwrap();
    fs::write(root.path().join("directory/secret"), b"public").unwrap();
    let running = Arc::new(AtomicBool::new(true));
    let flag = running.clone();
    let root_path = root.path().to_owned();
    let outside_path = outside.path().to_owned();
    let swapper = std::thread::spawn(move || {
        while flag.load(Ordering::Relaxed) {
            fs::rename(root_path.join("directory"), root_path.join("parked")).unwrap();
            std::os::unix::fs::symlink(&outside_path, root_path.join("directory")).unwrap();
            fs::remove_file(root_path.join("directory")).unwrap();
            fs::rename(root_path.join("parked"), root_path.join("directory")).unwrap();
        }
    });
    let mut leaked = false;
    for _ in 0..2000 {
        if let Ok(mut file) = storage.read(&path("/directory/secret")) {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            leaked |= bytes != b"public";
        }
    }
    running.store(false, Ordering::Relaxed);
    swapper.join().unwrap();
    assert!(!leaked);
}

#[test]
fn empty_files_unknown_lengths_and_chunk_limits() {
    let (root, storage) = setup();
    upload(&storage, "/empty", b"", Policy::Reject).unwrap();
    let mut writer = storage.begin_upload(&path("/unknown"), None).unwrap();
    writer.write_chunk(b"data").unwrap();
    writer.publish(Policy::Reject).unwrap();
    assert_eq!(fs::read(root.path().join("unknown")).unwrap(), b"data");
    let mut writer = storage.begin_upload(&path("/oversized"), None).unwrap();
    assert_eq!(writer.write_chunk(&[0; 65537]), Err(Error::UploadFailed));
    assert_eq!(writer.publish(Policy::Reject), Err(Error::UploadFailed));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[cfg(windows)]
#[test]
fn windows_file_reparse_point_is_denied() {
    let (root, storage) = setup();
    fs::write(root.path().join("real"), b"safe").unwrap();
    std::os::windows::fs::symlink_file(root.path().join("real"), root.path().join("link"))
        .expect("Windows reparse tests require Developer Mode or elevated runner");
    assert!(storage.read(&path("/link")).is_err());
    assert_eq!(
        storage.metadata(&path("/link")).unwrap_err(),
        Error::UnsupportedEntry
    );
    assert_eq!(
        upload(&storage, "/link", b"bad", Policy::Replace),
        Err(Error::UnsupportedEntry)
    );
    assert_eq!(fs::read(root.path().join("real")).unwrap(), b"safe");
}
