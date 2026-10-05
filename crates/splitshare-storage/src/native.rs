//! Platform atomic no-clobber rename. Arguments are validated single components.
use cap_std::fs::Dir;
use std::io;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn rename_noreplace(from: &Dir, old: &str, to: &Dir, new: &str) -> io::Result<()> {
    rustix::fs::renameat_with(from, old, to, new, rustix::fs::RenameFlags::NOREPLACE)
        .map_err(Into::into)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub(super) fn rename_noreplace(_from: &Dir, _old: &str, _to: &Dir, _new: &str) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

#[cfg(windows)]
pub(super) fn rename_noreplace(from: &Dir, old: &str, to: &Dir, new: &str) -> io::Result<()> {
    rename_windows(from, old, to, new, false)
}

#[cfg(windows)]
pub(super) fn rename_windows(
    from: &Dir,
    old: &str,
    to: &Dir,
    new: &str,
    replace: bool,
) -> io::Result<()> {
    use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt, OpenOptionsMaybeDirExt};
    use cap_std::fs::{MetadataExt, OpenOptions, OpenOptionsExt};
    use std::{
        mem::{offset_of, size_of},
        os::windows::io::AsRawHandle,
    };
    use windows_sys::{
        Wdk::Storage::FileSystem::{
            FILE_RENAME_INFORMATION, FileRenameInformation, NtSetInformationFile,
        },
        Win32::{
            Foundation::RtlNtStatusToDosError,
            Storage::FileSystem::{DELETE, FILE_READ_ATTRIBUTES},
            System::IO::IO_STATUS_BLOCK,
        },
    };
    let mut options = OpenOptions::new();
    options
        .access_mode(DELETE | FILE_READ_ATTRIBUTES)
        .maybe_dir(true)
        .follow(FollowSymlinks::No);
    let source = from.open_with(old, &options)?;
    if source.metadata()?.file_attributes() & 0x400 != 0 {
        return Err(io::Error::from(io::ErrorKind::PermissionDenied));
    }
    let name: Vec<u16> = new.encode_utf16().collect();
    let bytes = size_of::<FILE_RENAME_INFORMATION>() + name.len() * 2;
    // usize storage provides the alignment required by FILE_RENAME_INFORMATION's HANDLE.
    let mut buffer = vec![0usize; bytes.div_ceil(size_of::<usize>())];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    // SAFETY: allocation is aligned, zeroed, and large enough for the struct and
    // UTF-16 tail. Both handles remain borrowed and live throughout the OS call.
    // `new` is a validated single component, never an absolute or stream path.
    unsafe {
        (*info).Anonymous.ReplaceIfExists = replace;
        (*info).RootDirectory = to.as_raw_handle();
        (*info).FileNameLength = (name.len() * 2) as u32;
        let tail = buffer
            .as_mut_ptr()
            .cast::<u8>()
            .add(offset_of!(FILE_RENAME_INFORMATION, FileName))
            .cast::<u16>();
        std::ptr::copy_nonoverlapping(name.as_ptr(), tail, name.len());
        // Use the NT interface directly: the destination is relative to a held
        // directory capability, not a DOS path interpreted by the Win32 wrapper.
        // cap-std opens synchronous handles, so the stack status block remains
        // valid for the entire operation. NT errors are not GetLastError values.
        let mut io_status = IO_STATUS_BLOCK::default();
        let status = NtSetInformationFile(
            source.as_raw_handle(),
            &mut io_status,
            info.cast(),
            bytes as u32,
            FileRenameInformation,
        );
        if status < 0 {
            return Err(io::Error::from_raw_os_error(
                RtlNtStatusToDosError(status) as i32
            ));
        }
    }
    Ok(())
}

pub(super) fn rename_replace(from: &Dir, old: &str, to: &Dir, new: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        rename_windows(from, old, to, new, true)
    }
    #[cfg(not(windows))]
    {
        from.rename(old, to, new)
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn relative_rename_preserves_conflicts_and_supports_replacement() {
        let root = tempfile::tempdir().unwrap();
        let dir = Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).unwrap();
        dir.create_dir("destination").unwrap();
        let destination = dir.open_dir("destination").unwrap();
        dir.write("source", b"new").unwrap();
        destination.write("existing", b"old").unwrap();

        let error = rename_noreplace(&dir, "source", &destination, "existing").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists, "{error:?}");
        assert_eq!(dir.read("source").unwrap(), b"new");
        assert_eq!(destination.read("existing").unwrap(), b"old");

        rename_noreplace(&dir, "source", &destination, "資料.txt")
            .expect("handle-relative no-clobber rename failed");
        assert!(!dir.try_exists("source").unwrap());
        assert_eq!(destination.read("資料.txt").unwrap(), b"new");
        rename_replace(&destination, "資料.txt", &destination, "existing")
            .expect("handle-relative replacement failed");
        assert_eq!(destination.read("existing").unwrap(), b"new");

        dir.create_dir("folder").unwrap();
        rename_noreplace(&dir, "folder", &destination, "moved")
            .expect("handle-relative directory rename failed");
        assert!(destination.open_dir("moved").is_ok());
    }
}
