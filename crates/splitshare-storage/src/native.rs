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
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_READ_ATTRIBUTES, FILE_RENAME_INFO, FileRenameInfo, SetFileInformationByHandle,
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
    let bytes = size_of::<FILE_RENAME_INFO>() + name.len() * 2;
    // usize storage provides the alignment required by FILE_RENAME_INFO's HANDLE.
    let mut buffer = vec![0usize; bytes.div_ceil(size_of::<usize>())];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
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
            .add(offset_of!(FILE_RENAME_INFO, FileName))
            .cast::<u16>();
        std::ptr::copy_nonoverlapping(name.as_ptr(), tail, name.len());
        if SetFileInformationByHandle(
            source.as_raw_handle(),
            FileRenameInfo,
            info.cast(),
            bytes as u32,
        ) == 0
        {
            return Err(io::Error::last_os_error());
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
