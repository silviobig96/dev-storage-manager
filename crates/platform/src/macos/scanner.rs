//! Descriptor-relative observations, not an atomic containment snapshot. A
//! caller retains parents and verifies their links before continuing children;
//! a move between checks can briefly outlive the original pathname membership.
use super::identity::{self, wire_identity, ROOT_ATTRIBUTES, VOLUME_ATTRIBUTES};
use dev_storage_core::{scanner::*, workspaces::StoredWorkspace};
use std::{
    ffi::{CStr, CString, OsStr, OsString},
    io,
    mem::MaybeUninit,
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
        unix::ffi::{OsStrExt, OsStringExt},
    },
    path::Path,
    ptr::NonNull,
};

type ScanResult<T> = Result<T, ScanIoError>;

fn diagnostic(code: ScanDiagnosticCode) -> ScanIoError {
    ScanIoError::Diagnostic(code)
}
fn unsupported() -> ScanIoError {
    diagnostic(ScanDiagnosticCode::UnsupportedFilesystem)
}
fn changed() -> ScanIoError {
    diagnostic(ScanDiagnosticCode::ChangedDuringScan)
}

/// Includes calls nested inside a cursor operation. The owned result is dropped
/// if the after-call checkpoint stops work (including open/fdopendir results).
fn call<T>(control: &dyn ScanIoControl, action: impl FnOnce() -> ScanResult<T>) -> ScanResult<T> {
    control.checkpoint().map_err(ScanIoError::Stopped)?;
    #[cfg(test)]
    tests::record(tests::Event::Call);
    let result = action();
    control.checkpoint().map_err(ScanIoError::Stopped)?;
    result
}

fn io_error(error: io::Error, root: bool) -> ScanIoError {
    use ScanDiagnosticCode::*;
    diagnostic(match error.raw_os_error() {
        Some(libc::EMFILE | libc::ENFILE) => {
            return ScanIoError::Stopped(StopReason::DescriptorLimit);
        }
        Some(libc::ENOENT) => {
            if root {
                RootMissing
            } else {
                EntryMissing
            }
        }
        Some(libc::EACCES | libc::EPERM) => {
            if root {
                RootDenied
            } else {
                EntryDenied
            }
        }
        Some(libc::ELOOP | libc::ENOTDIR) => {
            if root {
                RootChanged
            } else {
                ChangedDuringScan
            }
        }
        _ => UnsupportedFilesystem,
    })
}

fn open(path: &Path, flags: i32, control: &dyn ScanIoControl) -> ScanResult<OwnedFd> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| unsupported())?;
    call(control, || {
        // SAFETY: valid NUL-terminated path; flags never create or write files.
        let raw = unsafe { libc::open(path.as_ptr(), flags | libc::O_CLOEXEC) };
        if raw < 0 {
            return Err(io_error(io::Error::last_os_error(), true));
        }
        #[cfg(test)]
        tests::record(tests::Event::Open { fd: raw, flags });
        // SAFETY: unique successful open; owns and closes the descriptor.
        Ok(unsafe { OwnedFd::from_raw_fd(raw) })
    })
}

fn stat(fd: RawFd, name: Option<&CStr>, control: &dyn ScanIoControl) -> ScanResult<libc::stat> {
    call(control, || {
        let mut value = MaybeUninit::<libc::stat>::uninit();
        // SAFETY: output has the exact native size. A child is a checked single
        // component; AT_SYMLINK_NOFOLLOW observes the link, never its target.
        #[cfg(test)]
        if let Some(name) = name {
            tests::record(tests::Event::MetadataAt {
                fd,
                name: name.to_bytes().to_vec(),
                flags: libc::AT_SYMLINK_NOFOLLOW as u32,
            });
        }
        let result = unsafe {
            match name {
                Some(name) => libc::fstatat(
                    fd,
                    name.as_ptr(),
                    value.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                ),
                None => libc::fstat(fd, value.as_mut_ptr()),
            }
        };
        if result < 0 {
            return Err(io_error(io::Error::last_os_error(), false));
        }
        // SAFETY: successful stat initialized the whole native value.
        Ok(unsafe { value.assume_init() })
    })
}

fn attributes(
    fd: RawFd,
    name: Option<&CStr>,
    common: u32,
    volume: u32,
    directory: u32,
    extended: u32,
    control: &dyn ScanIoControl,
) -> ScanResult<[u8; 128]> {
    call(control, || {
        let mut attrs = libc::attrlist {
            bitmapcount: libc::ATTR_BIT_MAP_COUNT,
            reserved: 0,
            commonattr: common,
            volattr: volume,
            dirattr: directory,
            fileattr: 0,
            forkattr: extended,
        };
        let mut bytes = [0; 128];
        let options = libc::FSOPT_REPORT_FULLSIZE
            | if extended != 0 {
                libc::FSOPT_ATTR_CMN_EXTENDED
            } else {
                0
            };
        #[cfg(test)]
        if let Some(name) = name {
            tests::record(tests::Event::MetadataAt {
                fd,
                name: name.to_bytes().to_vec(),
                flags: libc::FSOPT_NOFOLLOW,
            });
        }
        // SAFETY: byte buffer and attrlist have their declared size; decoded by
        // checked slices only. Child queries explicitly do not follow links.
        let result = unsafe {
            match name {
                Some(name) => libc::getattrlistat(
                    fd,
                    name.as_ptr(),
                    (&mut attrs as *mut libc::attrlist).cast(),
                    bytes.as_mut_ptr().cast(),
                    bytes.len(),
                    (options | libc::FSOPT_NOFOLLOW) as libc::c_ulong,
                ),
                None => libc::fgetattrlist(
                    fd,
                    (&mut attrs as *mut libc::attrlist).cast(),
                    bytes.as_mut_ptr().cast(),
                    bytes.len(),
                    options,
                ),
            }
        };
        if result < 0 {
            return Err(io_error(io::Error::last_os_error(), false));
        }
        #[cfg(test)]
        if extended != 0 {
            tests::alter_scope(&mut bytes);
        }
        Ok(bytes)
    })
}

fn volume(fd: RawFd, control: &dyn ScanIoControl) -> ScanResult<[u8; 16]> {
    let bytes = attributes(
        fd,
        None,
        libc::ATTR_CMN_RETURNED_ATTRS,
        VOLUME_ATTRIBUTES,
        0,
        0,
        control,
    )?;
    let (uuid, persistent) = identity::decode_volume(&bytes).map_err(|_| unsupported())?;
    if !persistent || uuid == [0; 16] {
        return Err(unsupported());
    }
    Ok(uuid)
}

fn object_id(value: &libc::stat, volume_uuid: [u8; 16]) -> ScanResult<ObjectIdentity> {
    if value.st_ino == 0 || !(0..1_000_000_000).contains(&value.st_birthtime_nsec) {
        return Err(unsupported());
    }
    Ok(ObjectIdentity {
        volume_uuid,
        file_id: value.st_ino,
        birth_seconds: value.st_birthtime,
        birth_nanoseconds: value.st_birthtime_nsec,
    })
}

fn descriptor_identity(fd: RawFd, control: &dyn ScanIoControl) -> ScanResult<ObjectIdentity> {
    let bytes = attributes(fd, None, ROOT_ATTRIBUTES, 0, 0, 0, control)?;
    let (directory, file_id, birth_seconds, birth_nanoseconds) =
        identity::decode_root(&bytes).map_err(|_| unsupported())?;
    if !directory {
        return Err(changed());
    }
    if file_id == 0 || !(0..1_000_000_000).contains(&birth_nanoseconds) {
        return Err(unsupported());
    }
    Ok(ObjectIdentity {
        volume_uuid: volume(fd, control)?,
        file_id,
        birth_seconds,
        birth_nanoseconds,
    })
}

pub(super) trait AccountHome: Send + Sync {
    fn identity(&self, control: &dyn ScanIoControl) -> ScanResult<ObjectIdentity>;
}
pub(super) struct NativeAccountHome;
impl AccountHome for NativeAccountHome {
    fn identity(&self, control: &dyn ScanIoControl) -> ScanResult<ObjectIdentity> {
        // getpwuid_r uses account metadata, never HOME or a project-root convention.
        // SAFETY: getuid takes no pointers and only reads the process credential.
        let uid = call(control, || Ok(unsafe { libc::getuid() }))?;
        let home = call(control, || {
            let mut value = MaybeUninit::<libc::passwd>::uninit();
            let mut buffer = [0u8; 16 * 1024];
            let mut result = std::ptr::null_mut();
            // SAFETY: result points into the supplied buffer, kept alive while copied.
            let error = unsafe {
                libc::getpwuid_r(
                    uid,
                    value.as_mut_ptr(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    &mut result,
                )
            };
            if error != 0 || result.is_null() {
                return Err(unsupported());
            }
            // SAFETY: getpwuid_r succeeded and returned a passwd in live storage.
            let value = unsafe { value.assume_init() };
            if value.pw_dir.is_null() {
                return Err(unsupported());
            }
            // SAFETY: pw_dir is a terminated string in getpwuid_r's live buffer.
            let bytes = unsafe { CStr::from_ptr(value.pw_dir) }.to_bytes();
            let home = std::path::PathBuf::from(OsString::from_vec(bytes.to_vec()));
            if !home.is_absolute() {
                return Err(unsupported());
            }
            Ok(home)
        })?;
        // Only this metadata guard opens the account home. No enumeration/content.
        let fd = open(&home, libc::O_EVTONLY, control)?;
        descriptor_identity(fd.as_raw_fd(), control)
    }
}

pub(super) struct NativeFilesystem<H = NativeAccountHome> {
    home: H,
}
impl Default for NativeFilesystem {
    fn default() -> Self {
        Self {
            home: NativeAccountHome,
        }
    }
}

impl<H: AccountHome> ScanFilesystem for NativeFilesystem<H> {
    fn anchor_root(
        &self,
        root: &StoredWorkspace,
        control: &dyn ScanIoControl,
    ) -> ScanResult<Box<dyn ScanDirectory>> {
        self.anchor(root, control).map_err(|error| match error {
            ScanIoError::Diagnostic(ScanDiagnosticCode::EntryMissing) => {
                diagnostic(ScanDiagnosticCode::RootMissing)
            }
            ScanIoError::Diagnostic(ScanDiagnosticCode::EntryDenied) => {
                diagnostic(ScanDiagnosticCode::RootDenied)
            }
            ScanIoError::Diagnostic(ScanDiagnosticCode::ChangedDuringScan) => {
                diagnostic(ScanDiagnosticCode::RootChanged)
            }
            other => other,
        })
    }
}

impl<H: AccountHome> NativeFilesystem<H> {
    fn anchor(
        &self,
        root: &StoredWorkspace,
        control: &dyn ScanIoControl,
    ) -> ScanResult<Box<dyn ScanDirectory>> {
        let root = &root.root;
        if !Path::new(&root.selected_path).is_absolute()
            || !Path::new(&root.canonical_path).is_absolute()
        {
            return Err(diagnostic(ScanDiagnosticCode::RootChanged));
        }
        let selected = open(Path::new(&root.selected_path), libc::O_EVTONLY, control)?;
        let expected = descriptor_identity(selected.as_raw_fd(), control)?;
        if wire_identity(expected) != root.identity.as_str() {
            return Err(diagnostic(ScanDiagnosticCode::RootChanged));
        }
        let canonical = open(
            Path::new(&root.canonical_path),
            libc::O_EVTONLY | libc::O_NOFOLLOW | libc::O_DIRECTORY,
            control,
        )?;
        if descriptor_identity(canonical.as_raw_fd(), control)? != expected {
            return Err(diagnostic(ScanDiagnosticCode::RootChanged));
        }
        let initial = stat(canonical.as_raw_fd(), None, control)?;
        let mount = mount_info(canonical.as_raw_fd(), control)?;
        let facts = scope_facts(
            canonical.as_raw_fd(),
            None,
            &initial,
            initial.st_dev,
            control,
        )?;
        if facts.mount == MountFact::Boundary || self.home.identity(control)? == expected {
            return Err(diagnostic(ScanDiagnosticCode::RootTooBroad));
        }
        allow_directory(&facts)?;
        drop(selected);
        let fd = open(
            Path::new(&root.canonical_path),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW,
            control,
        )?;
        let cursor = Cursor::new(fd, expected, mount, control)?;
        // Canonical metadata descriptor stayed alive through the acquisition.
        if object_id(&initial, expected.volume_uuid)? != expected {
            return Err(diagnostic(ScanDiagnosticCode::RootChanged));
        }
        Ok(Box::new(cursor))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MountIdentity([i32; 2]);
fn mount_info(fd: RawFd, control: &dyn ScanIoControl) -> ScanResult<MountIdentity> {
    call(control, || {
        let mut value = MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: correctly sized native output and live descriptor.
        if unsafe { libc::fstatfs(fd, value.as_mut_ptr()) } < 0 {
            return Err(unsupported());
        }
        // SAFETY: initialized by successful fstatfs.
        let value = unsafe { value.assume_init() };
        // SAFETY: Darwin fsid_t is two initialized i32 values; transmute checks size.
        let id = unsafe { std::mem::transmute::<libc::fsid_t, [i32; 2]>(value.f_fsid) };
        supported_mount(value.f_flags, id)
    })
}

fn supported_mount(flags: u32, id: [i32; 2]) -> ScanResult<MountIdentity> {
    if flags & libc::MNT_LOCAL as u32 == 0 || id == [0; 2] {
        return Err(unsupported());
    }
    Ok(MountIdentity(id))
}

fn kind(value: &libc::stat) -> EntryKind {
    match value.st_mode & libc::S_IFMT {
        libc::S_IFDIR => EntryKind::Directory,
        libc::S_IFREG => EntryKind::RegularFile,
        libc::S_IFLNK => EntryKind::Symlink,
        _ => EntryKind::Special,
    }
}

fn component(name: &OsStr) -> ScanResult<CString> {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes == b"." || bytes == b".." || bytes.contains(&b'/') {
        return Err(changed());
    }
    CString::new(bytes).map_err(|_| changed())
}

fn native_name(entry: &libc::dirent) -> ScanResult<OsString> {
    let len = usize::from(entry.d_namlen);
    if len == 0 || len >= entry.d_name.len() || entry.d_name[len] != 0 {
        return Err(changed());
    }
    let bytes: Vec<u8> = entry.d_name[..len].iter().map(|byte| *byte as u8).collect();
    Ok(OsString::from_vec(bytes))
}

// These flags are not exported by locked libc; Darwin sys/stat.h defines them.
const EF_IS_SYNC_ROOT: u64 = 0x4;
const SF_DATALESS: u32 = 0x4000_0000;

fn scope_facts(
    fd: RawFd,
    name: Option<&CStr>,
    value: &libc::stat,
    device: libc::dev_t,
    control: &dyn ScanIoControl,
) -> ScanResult<ReadFacts> {
    let kind = kind(value);
    let facts = ReadFacts {
        kind,
        in_read_scope: true,
        filesystem_supported: true,
        mount: if value.st_dev == device {
            MountFact::Same
        } else {
            MountFact::Boundary
        },
        cloud: CloudFact::Unknown,
        root_too_broad: false,
    };
    if matches!(kind, EntryKind::Symlink | EntryKind::Special) || facts.mount == MountFact::Boundary
    {
        return Ok(facts);
    }
    let directory = if kind == EntryKind::Directory {
        libc::ATTR_DIR_MOUNTSTATUS
    } else {
        0
    };
    let bytes = match attributes(
        fd,
        name,
        ROOT_ATTRIBUTES,
        0,
        directory,
        libc::ATTR_CMNEXT_EXT_FLAGS,
        control,
    ) {
        Ok(bytes) => bytes,
        Err(ScanIoError::Diagnostic(ScanDiagnosticCode::UnsupportedFilesystem)) => {
            return scope_observation(facts, value, &[])
        }
        Err(error) => return Err(error),
    };
    scope_observation(facts, value, &bytes)
}

fn scope_observation(
    mut facts: ReadFacts,
    value: &libc::stat,
    bytes: &[u8],
) -> ScanResult<ReadFacts> {
    let kind = facts.kind;
    let Some(ScopeAttributes {
        object: observed,
        mount_status,
        extended_flags: extended,
    }) = decode_scope(bytes, kind == EntryKind::Directory)
    else {
        if kind == EntryKind::Directory {
            facts.mount = MountFact::Unknown;
        }
        return Ok(facts);
    };
    if observed
        != (
            kind == EntryKind::Directory,
            value.st_ino,
            value.st_birthtime,
            value.st_birthtime_nsec,
        )
    {
        return Err(changed());
    }
    facts.cloud = if extended & EF_IS_SYNC_ROOT != 0 {
        CloudFact::SyncRoot
    } else if value.st_flags & SF_DATALESS != 0 {
        CloudFact::Dataless
    } else {
        CloudFact::Materialized
    };
    if mount_status != 0 {
        facts.mount = MountFact::Boundary;
    }
    Ok(facts)
}

#[derive(Debug, PartialEq, Eq)]
struct ScopeAttributes {
    object: (bool, u64, i64, i64),
    mount_status: u32,
    extended_flags: u64,
}

fn decode_scope(bytes: &[u8], directory: bool) -> Option<ScopeAttributes> {
    let length = if directory { 64 } else { 60 };
    let get32 = |offset| identity::array(bytes, offset).ok().map(u32::from_ne_bytes);
    if get32(0)? as usize != length
        || bytes.len() < length
        || get32(4)? != ROOT_ATTRIBUTES
        || get32(8)? != 0
        || get32(12)?
            != if directory {
                libc::ATTR_DIR_MOUNTSTATUS
            } else {
                0
            }
        || get32(16)? != 0
        || get32(20)? != libc::ATTR_CMNEXT_EXT_FLAGS
        || get32(24)? != if directory { 2 } else { 1 }
    {
        return None;
    }
    let root = (
        get32(24)? == 2,
        u64::from_ne_bytes(identity::array(bytes, 44).ok()?),
        i64::from_ne_bytes(identity::array(bytes, 28).ok()?),
        i64::from_ne_bytes(identity::array(bytes, 36).ok()?),
    );
    let mount = if directory { get32(52)? } else { 0 };
    let ext = u64::from_ne_bytes(identity::array(bytes, if directory { 56 } else { 52 }).ok()?);
    Some(ScopeAttributes {
        object: root,
        mount_status: mount,
        extended_flags: ext,
    })
}

fn allow_directory(facts: &ReadFacts) -> ScanResult<()> {
    match ScanReadPolicy::evaluate(facts) {
        ReadDecision::EnumerateDirectory => Ok(()),
        ReadDecision::Skip(reason) => Err(diagnostic(reason.into())),
        _ => Err(changed()),
    }
}

fn allocation(blocks: Option<i64>, cloud: CloudFact) -> ScanResult<Option<u64>> {
    if cloud != CloudFact::Materialized {
        return Ok(None);
    }
    blocks
        .and_then(|v| u64::try_from(v).ok())
        .map(|v| {
            v.checked_mul(512)
                .ok_or(ScanIoError::Stopped(StopReason::QuantityOverflow))
        })
        .transpose()
}

struct DirectoryHandle {
    pointer: NonNull<libc::DIR>,
    fd: RawFd,
}
// SAFETY: the DIR stream has a unique owner; no concurrent access is exposed.
unsafe impl Send for DirectoryHandle {}
impl Drop for DirectoryHandle {
    fn drop(&mut self) {
        #[cfg(test)]
        tests::record(tests::Event::Closed(self.fd));
        // SAFETY: uniquely owns this live DIR and its fd. Cleanup cannot be cancelled.
        unsafe {
            libc::closedir(self.pointer.as_ptr());
        }
    }
}

struct Cursor {
    directory: DirectoryHandle,
    identity: ObjectIdentity,
    mount: MountIdentity,
    initial: libc::stat,
    ended: bool,
}

impl Cursor {
    fn new(
        fd: OwnedFd,
        expected: ObjectIdentity,
        mount: MountIdentity,
        control: &dyn ScanIoControl,
    ) -> ScanResult<Self> {
        let initial = stat(fd.as_raw_fd(), None, control)?;
        if kind(&initial) != EntryKind::Directory
            || descriptor_identity(fd.as_raw_fd(), control)? != expected
        {
            return Err(changed());
        }
        if mount_info(fd.as_raw_fd(), control)? != mount {
            return Err(diagnostic(ScanDiagnosticCode::MountExcluded));
        }
        allow_directory(&scope_facts(
            fd.as_raw_fd(),
            None,
            &initial,
            initial.st_dev,
            control,
        )?)?;
        let directory = call(control, || {
            // SAFETY: owned directory fd; only fdopendir success transfers ownership.
            let pointer = unsafe { libc::fdopendir(fd.as_raw_fd()) };
            let pointer =
                NonNull::new(pointer).ok_or_else(|| io_error(io::Error::last_os_error(), false))?;
            Ok(DirectoryHandle {
                pointer,
                fd: fd.into_raw_fd(),
            })
        })?;
        Ok(Self {
            directory,
            identity: expected,
            mount,
            initial,
            ended: false,
        })
    }
    fn active(&self, control: &dyn ScanIoControl) -> ScanResult<()> {
        let current = stat(self.directory.fd, None, control)?;
        if object_id(&current, self.identity.volume_uuid)? != self.identity
            || current.st_dev != self.initial.st_dev
            || (
                current.st_mtime,
                current.st_mtime_nsec,
                current.st_ctime,
                current.st_ctime_nsec,
            ) != (
                self.initial.st_mtime,
                self.initial.st_mtime_nsec,
                self.initial.st_ctime,
                self.initial.st_ctime_nsec,
            )
        {
            return Err(changed());
        }
        if mount_info(self.directory.fd, control)? != self.mount {
            return Err(changed());
        }
        allow_directory(&scope_facts(
            self.directory.fd,
            None,
            &current,
            self.initial.st_dev,
            control,
        )?)
    }
}

impl ScanDirectory for Cursor {
    fn identity(&self) -> ObjectIdentity {
        self.identity
    }
    fn next_entry(&mut self, control: &dyn ScanIoControl) -> ScanResult<Option<ScanEntry>> {
        self.active(control)?;
        if self.ended {
            return Ok(None);
        }
        loop {
            let name = call(control, || {
                #[cfg(test)]
                tests::record(tests::Event::Enumerate(self.directory.fd));
                // SAFETY: unique live DIR; errno is thread-local. Copy bytes before
                // the next readdir can invalidate the returned entry.
                unsafe {
                    *libc::__error() = 0;
                    let entry = libc::readdir(self.directory.pointer.as_ptr());
                    if entry.is_null() {
                        let error = *libc::__error();
                        return if error == 0 {
                            Ok(None)
                        } else {
                            Err(io_error(io::Error::from_raw_os_error(error), false))
                        };
                    }
                    native_name(&*entry).map(Some)
                }
            })?;
            let Some(name) = name else {
                self.ended = true;
                return Ok(None);
            };
            if name == "." || name == ".." {
                continue;
            }
            let native = component(&name)?;
            let value = stat(self.directory.fd, Some(&native), control)?;
            let facts = scope_facts(
                self.directory.fd,
                Some(&native),
                &value,
                self.initial.st_dev,
                control,
            )?;
            let identity = if facts.mount == MountFact::Same {
                Some(object_id(&value, self.identity.volume_uuid)?)
            } else {
                None
            };
            let logical_bytes = if facts.kind == EntryKind::RegularFile {
                u64::try_from(value.st_size).ok()
            } else {
                None
            };
            let allocated_bytes = if facts.kind == EntryKind::RegularFile {
                allocation(Some(value.st_blocks), facts.cloud)?
            } else {
                None
            };
            return Ok(Some(ScanEntry {
                name,
                identity,
                facts,
                logical_bytes,
                allocated_bytes,
            }));
        }
    }
    fn open_child(
        &mut self,
        entry: &ScanEntry,
        control: &dyn ScanIoControl,
    ) -> ScanResult<Box<dyn ScanDirectory>> {
        allow_directory(&entry.facts)?;
        self.active(control)?;
        let expected = entry.identity.ok_or_else(unsupported)?;
        let name = component(&entry.name)?;
        let fd = call(control, || {
            #[cfg(test)]
            tests::record(tests::Event::OpenAt {
                parent: self.directory.fd,
                name: name.to_bytes().to_vec(),
                flags: libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            });
            // SAFETY: checked single name, live parent. Only directories are opened,
            // and O_NOFOLLOW rejects symlink substitution atomically with openat.
            let raw = unsafe {
                libc::openat(
                    self.directory.fd,
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if raw < 0 {
                return Err(io_error(io::Error::last_os_error(), false));
            }
            #[cfg(test)]
            tests::record(tests::Event::Open {
                fd: raw,
                flags: libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            });
            // SAFETY: uniquely owned descriptor from successful openat.
            Ok(unsafe { OwnedFd::from_raw_fd(raw) })
        })?;
        let child = Cursor::new(fd, expected, self.mount, control)?;
        self.verify_link(entry, &child.identity, control)?;
        Ok(Box::new(child))
    }
    fn verify_link(
        &self,
        entry: &ScanEntry,
        child: &ObjectIdentity,
        control: &dyn ScanIoControl,
    ) -> ScanResult<()> {
        self.active(control)?;
        let name = component(&entry.name)?;
        let value = stat(self.directory.fd, Some(&name), control).map_err(|e| match e {
            ScanIoError::Diagnostic(ScanDiagnosticCode::EntryMissing) => changed(),
            other => other,
        })?;
        if kind(&value) != EntryKind::Directory
            || value.st_dev != self.initial.st_dev
            || object_id(&value, self.identity.volume_uuid)? != *child
            || entry.identity != Some(*child)
        {
            return Err(changed());
        }
        allow_directory(&scope_facts(
            self.directory.fd,
            Some(&name),
            &value,
            self.initial.st_dev,
            control,
        )?)
    }
    fn finish(&mut self, control: &dyn ScanIoControl) -> ScanResult<()> {
        self.active(control)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dev_storage_core::workspaces::{StoredWorkspace, WorkspaceId};
    use std::{
        fs,
        os::unix::{ffi::OsStrExt, fs::symlink},
        path::Path,
    };

    #[derive(Debug, Clone)]
    pub(super) enum Event {
        Call,
        Open {
            fd: RawFd,
            flags: i32,
        },
        OpenAt {
            parent: RawFd,
            name: Vec<u8>,
            flags: i32,
        },
        MetadataAt {
            fd: RawFd,
            name: Vec<u8>,
            flags: u32,
        },
        Enumerate(RawFd),
        Closed(RawFd),
    }
    thread_local! { static EVENTS: std::cell::RefCell<Vec<Event>> = const { std::cell::RefCell::new(Vec::new()) }; }
    type ScopePatch = fn(&mut [u8; 128]);
    thread_local! { static SCOPE_PATCH: std::cell::RefCell<Option<ScopePatch>> = const { std::cell::RefCell::new(None) }; }
    pub(super) fn alter_scope(bytes: &mut [u8; 128]) {
        SCOPE_PATCH.with(|patch| {
            if let Some(patch) = patch.borrow_mut().take() {
                patch(bytes);
            }
        });
    }
    pub(super) fn record(event: Event) {
        EVENTS.with(|events| events.borrow_mut().push(event));
    }
    fn take_events() -> Vec<Event> {
        EVENTS.with(|events| std::mem::take(&mut *events.borrow_mut()))
    }

    fn assert_released(fd: RawFd, expected: ObjectIdentity) {
        // Other parallel tests may reuse the numeric fd after our close. Check
        // whether it still refers to this fixture object, not merely F_GETFD.
        let mut value = MaybeUninit::<libc::stat>::uninit();
        if unsafe { libc::fstat(fd, value.as_mut_ptr()) } == 0 {
            let value = unsafe { value.assume_init() };
            assert_ne!(
                (value.st_ino, value.st_birthtime, value.st_birthtime_nsec),
                (
                    expected.file_id,
                    expected.birth_seconds,
                    expected.birth_nanoseconds
                ),
                "fixture descriptor leaked"
            );
        } else {
            assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::EBADF));
        }
    }

    struct StopAt {
        at: usize,
        calls: std::sync::atomic::AtomicUsize,
    }
    impl ScanIoControl for StopAt {
        fn checkpoint(&self) -> Result<(), StopReason> {
            let count = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            if count >= self.at {
                Err(StopReason::Cancelled)
            } else {
                Ok(())
            }
        }
    }

    struct FixtureHome;
    impl AccountHome for FixtureHome {
        fn identity(&self, _: &dyn ScanIoControl) -> ScanResult<ObjectIdentity> {
            Ok(ObjectIdentity {
                volume_uuid: [9; 16],
                file_id: 1,
                birth_seconds: 0,
                birth_nanoseconds: 0,
            })
        }
    }
    fn filesystem() -> NativeFilesystem<FixtureHome> {
        NativeFilesystem { home: FixtureHome }
    }

    struct Running;
    impl ScanIoControl for Running {
        fn checkpoint(&self) -> Result<(), StopReason> {
            Ok(())
        }
    }

    fn saved(path: &Path) -> StoredWorkspace {
        StoredWorkspace {
            id: WorkspaceId::try_from(1).unwrap(),
            root: super::super::workspaces::validate(path).unwrap(),
            enabled: true,
            created_at: 0,
            updated_at: 0,
        }
    }

    // Fails if anchoring ignores the saved identity or follows a retargeted selection.
    #[test]
    fn scan_root_anchor_revalidates_saved_object() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let selection = temp.path().join("selected");
        symlink(&root, &selection).unwrap();
        let stored = saved(&selection);
        assert!(filesystem().anchor_root(&stored, &Running).is_ok());
        fs::rename(&root, temp.path().join("moved")).unwrap();
        assert!(matches!(
            filesystem().anchor_root(&stored, &Running),
            Err(ScanIoError::Diagnostic(ScanDiagnosticCode::RootMissing))
        ));
        fs::create_dir(&root).unwrap();
        assert!(matches!(
            filesystem().anchor_root(&stored, &Running),
            Err(ScanIoError::Diagnostic(ScanDiagnosticCode::RootChanged))
        ));
    }

    #[test]
    fn scan_root_anchor_replaced_by_regular_file_is_root_changed() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let stored = saved(&root);
        fs::rename(&root, temp.path().join("moved")).unwrap();
        fs::write(&root, b"replacement").unwrap();
        take_events();
        assert!(matches!(
            filesystem().anchor_root(&stored, &Running),
            Err(ScanIoError::Diagnostic(ScanDiagnosticCode::RootChanged))
        ));
        assert!(!take_events()
            .iter()
            .any(|e| matches!(e, Event::Enumerate(_))));
    }

    #[test]
    fn scan_root_anchor_retargeted_selection_never_enumerates_new_target() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let other = temp.path().join("other");
        let alias = temp.path().join("alias");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&other).unwrap();
        symlink(&root, &alias).unwrap();
        let stored = saved(&alias);
        fs::rename(&alias, temp.path().join("old-alias")).unwrap();
        symlink(&other, &alias).unwrap();
        take_events();
        assert!(matches!(
            filesystem().anchor_root(&stored, &Running),
            Err(ScanIoError::Diagnostic(ScanDiagnosticCode::RootChanged))
        ));
        let events = take_events();
        assert!(!events.iter().any(|e| matches!(e, Event::Enumerate(_))));
        assert!(events
            .iter()
            .all(|e| !matches!(e, Event::Open { flags, .. } if flags & libc::O_EVTONLY == 0)));
    }

    // Removing no-follow or treating names as exclusions exposes a target or loses a directory.
    #[test]
    fn scan_cursor_no_follow_and_native_names() {
        use std::os::unix::ffi::OsStringExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let outside = temp.path().join("outside");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("external-sentinel"), b"never read").unwrap();
        for name in [".hidden", ".git", "node_modules", "Café 工具"] {
            fs::create_dir(root.join(name)).unwrap();
        }
        let native_name = std::ffi::OsString::from_vec(b"regular".to_vec());
        fs::write(root.join(&native_name), b"abc").unwrap();
        symlink(&outside, root.join("outside-link")).unwrap();
        symlink(".", root.join("loop")).unwrap();
        symlink("missing", root.join("broken")).unwrap();
        let fifo = std::ffi::CString::new(root.join("fifo").as_os_str().as_bytes()).unwrap();
        // SAFETY: controlled temporary fixture and a valid C string.
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        let mut cursor = filesystem().anchor_root(&saved(&root), &Running).unwrap();
        take_events();
        let mut names = Vec::new();
        let mut links = 0;
        let mut specials = 0;
        while let Some(entry) = cursor.next_entry(&Running).unwrap() {
            names.push(entry.name.clone());
            match entry.facts.kind {
                EntryKind::Directory => {
                    let mut child = cursor.open_child(&entry, &Running).unwrap();
                    assert!(child.next_entry(&Running).unwrap().is_none());
                    child.finish(&Running).unwrap();
                }
                EntryKind::Symlink => {
                    links += 1;
                    assert!(matches!(
                        cursor.open_child(&entry, &Running),
                        Err(ScanIoError::Diagnostic(ScanDiagnosticCode::SymlinkExcluded))
                    ));
                }
                EntryKind::Special => {
                    specials += 1;
                    assert!(matches!(
                        cursor.open_child(&entry, &Running),
                        Err(ScanIoError::Diagnostic(ScanDiagnosticCode::SpecialExcluded))
                    ));
                }
                EntryKind::RegularFile => assert_eq!(entry.logical_bytes, Some(3)),
            }
        }
        assert_eq!(names.len(), 9);
        assert!(names.contains(&native_name));
        assert_eq!((links, specials), (3, 1));
        cursor.finish(&Running).unwrap();
        let events = take_events();
        let mut child_opens = 0;
        let mut enumerated = std::collections::HashSet::new();
        let mut closed = std::collections::HashSet::new();
        for event in events {
            match event {
                Event::Open { flags, .. } => {
                    assert_ne!(flags & libc::O_DIRECTORY, 0, "regular/special content open")
                }
                Event::OpenAt {
                    parent,
                    name,
                    flags,
                } => {
                    child_opens += 1;
                    assert!(parent >= 0);
                    assert_ne!(flags & libc::O_NOFOLLOW, 0);
                    assert_ne!(flags & libc::O_DIRECTORY, 0);
                    assert!([
                        b".hidden".as_slice(),
                        b".git",
                        b"node_modules",
                        "Café 工具".as_bytes()
                    ]
                    .contains(&name.as_slice()));
                }
                Event::MetadataAt { fd, name, flags } => {
                    assert!(fd >= 0);
                    assert!(!name.contains(&b'/'));
                    assert_ne!(name, b"external-sentinel");
                    assert!(
                        flags == libc::AT_SYMLINK_NOFOLLOW as u32 || flags == libc::FSOPT_NOFOLLOW
                    );
                }
                Event::Enumerate(fd) => {
                    enumerated.insert(fd);
                }
                Event::Closed(fd) => {
                    closed.insert(fd);
                }
                Event::Call => {}
            }
        }
        assert_eq!(child_opens, 4);
        assert!(!enumerated.is_empty());
        assert!(!closed.is_empty());
    }

    // Fails if open_child trusts stale metadata, or verify_link trusts the held descriptor alone.
    #[test]
    fn scan_cursor_replacement_and_parent_move_are_observed() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("child")).unwrap();
        let mut cursor = filesystem().anchor_root(&saved(&root), &Running).unwrap();
        let entry = cursor.next_entry(&Running).unwrap().unwrap();
        fs::rename(root.join("child"), temp.path().join("moved")).unwrap();
        symlink(temp.path().join("moved"), root.join("child")).unwrap();
        assert!(matches!(
            cursor.open_child(&entry, &Running),
            Err(ScanIoError::Diagnostic(
                ScanDiagnosticCode::ChangedDuringScan
            ))
        ));

        let root2 = temp.path().join("root2");
        fs::create_dir(&root2).unwrap();
        fs::create_dir(root2.join("child")).unwrap();
        let mut parent = filesystem().anchor_root(&saved(&root2), &Running).unwrap();
        let entry = parent.next_entry(&Running).unwrap().unwrap();
        let child = parent.open_child(&entry, &Running).unwrap();
        fs::rename(root2.join("child"), temp.path().join("moved2")).unwrap();
        assert!(matches!(
            parent.verify_link(&entry, &child.identity(), &Running),
            Err(ScanIoError::Diagnostic(
                ScanDiagnosticCode::ChangedDuringScan
            ))
        ));
        assert!(matches!(
            parent.finish(&Running),
            Err(ScanIoError::Diagnostic(
                ScanDiagnosticCode::ChangedDuringScan
            ))
        ));
    }

    // Fails if st_size is substituted for allocation, or sparse content is read.
    #[test]
    fn scan_metadata_sparse_file_keeps_logical_and_allocated_distinct() {
        let temp = tempfile::tempdir().unwrap();
        fs::File::create(temp.path().join("sparse"))
            .unwrap()
            .set_len(16 * 1024 * 1024)
            .unwrap();
        let mut cursor = filesystem()
            .anchor_root(&saved(temp.path()), &Running)
            .unwrap();
        let entry = cursor.next_entry(&Running).unwrap().unwrap();
        assert_eq!(entry.logical_bytes, Some(16 * 1024 * 1024));
        assert!(entry.allocated_bytes.unwrap() < entry.logical_bytes.unwrap());
    }

    fn scope_bytes(directory: bool) -> [u8; 128] {
        let mut bytes = [0u8; 128];
        bytes[0..4].copy_from_slice(&(if directory { 64u32 } else { 60u32 }).to_ne_bytes());
        bytes[4..8].copy_from_slice(&ROOT_ATTRIBUTES.to_ne_bytes());
        bytes[12..16].copy_from_slice(
            &(if directory {
                libc::ATTR_DIR_MOUNTSTATUS
            } else {
                0
            })
            .to_ne_bytes(),
        );
        bytes[20..24].copy_from_slice(&libc::ATTR_CMNEXT_EXT_FLAGS.to_ne_bytes());
        bytes[24..28].copy_from_slice(&(if directory { 2u32 } else { 1u32 }).to_ne_bytes());
        bytes[28..36].copy_from_slice(&123i64.to_ne_bytes());
        bytes[36..44].copy_from_slice(&456i64.to_ne_bytes());
        bytes[44..52].copy_from_slice(&10u64.to_ne_bytes());
        bytes
    }

    // A corrupt/missing returned field must not turn into permission to enumerate.
    #[test]
    fn scan_metadata_scope_decoder_rejects_wrong_native_type_and_masks() {
        for directory in [true, false] {
            let valid = scope_bytes(directory);
            assert_eq!(
                decode_scope(&valid, directory),
                Some(ScopeAttributes {
                    object: (directory, 10, 123, 456),
                    mount_status: 0,
                    extended_flags: 0
                })
            );
            for offset in [0, 4, 12, 20, 24] {
                let mut bytes = valid;
                bytes[offset..offset + 4].copy_from_slice(&u32::MAX.to_ne_bytes());
                assert!(
                    decode_scope(&bytes, directory).is_none(),
                    "bad field at {offset}"
                );
            }
            for len in 0..if directory { 64 } else { 60 } {
                assert!(decode_scope(&valid[..len], directory).is_none());
            }
        }
    }

    // Missing/negative blocks are unknown; a representable native count must not wrap bytes.
    #[test]
    fn scan_metadata_allocation_unknown_and_overflow() {
        for blocks in [None, Some(-1)] {
            assert_eq!(allocation(blocks, CloudFact::Materialized), Ok(None));
        }
        assert_eq!(allocation(Some(3), CloudFact::Materialized), Ok(Some(1536)));
        assert_eq!(allocation(Some(0), CloudFact::Materialized), Ok(Some(0)));
        assert_eq!(
            allocation(Some(i64::MAX), CloudFact::Materialized),
            Err(ScanIoError::Stopped(StopReason::QuantityOverflow))
        );
        for cloud in [CloudFact::Dataless, CloudFact::SyncRoot, CloudFact::Unknown] {
            assert_eq!(allocation(Some(3), cloud), Ok(None));
        }
    }

    #[test]
    fn scan_metadata_injected_network_and_unknown_mount_are_unsupported() {
        assert_eq!(supported_mount(0, [1, 2]), Err(unsupported()));
        assert_eq!(
            supported_mount(libc::MNT_LOCAL as u32, [0, 0]),
            Err(unsupported())
        );
        assert_eq!(
            supported_mount(libc::MNT_LOCAL as u32, [1, 2]),
            Ok(MountIdentity([1, 2]))
        );
    }

    // Missing mount evidence must stay unknown; dataless/sync-root metadata cannot authorize traversal.
    #[test]
    fn scan_metadata_injected_cloud_and_mount_facts_fail_closed() {
        // SAFETY: stat is a C POD structure; this synthetic fixture sets the fields under test.
        let mut value: libc::stat = unsafe { std::mem::zeroed() };
        value.st_ino = 10;
        value.st_birthtime = 123;
        value.st_birthtime_nsec = 456;
        let facts = ReadFacts {
            kind: EntryKind::Directory,
            in_read_scope: true,
            filesystem_supported: true,
            mount: MountFact::Same,
            cloud: CloudFact::Unknown,
            root_too_broad: false,
        };
        let bytes = scope_bytes(true);
        let materialized = scope_observation(facts, &value, &bytes).unwrap();
        assert_eq!(materialized.cloud, CloudFact::Materialized);
        assert_eq!(
            ScanReadPolicy::evaluate(&materialized),
            ReadDecision::EnumerateDirectory
        );
        value.st_flags = SF_DATALESS;
        let dataless = scope_observation(facts, &value, &bytes).unwrap();
        assert_eq!(dataless.cloud, CloudFact::Dataless);
        assert_eq!(
            allow_directory(&dataless),
            Err(diagnostic(ScanDiagnosticCode::CloudNotMaterialized))
        );
        value.st_flags = 0;
        let mut sync = bytes;
        sync[56..64].copy_from_slice(&4u64.to_ne_bytes());
        let sync = scope_observation(facts, &value, &sync).unwrap();
        assert_eq!(sync.cloud, CloudFact::SyncRoot);
        assert_eq!(
            allow_directory(&sync),
            Err(diagnostic(ScanDiagnosticCode::CloudScopeUnsupported))
        );
        let mut mount = bytes;
        mount[52..56].copy_from_slice(&1u32.to_ne_bytes());
        let mount = scope_observation(facts, &value, &mount).unwrap();
        assert_eq!(mount.mount, MountFact::Boundary);
        assert_eq!(
            allow_directory(&mount),
            Err(diagnostic(ScanDiagnosticCode::MountExcluded))
        );
        let mut absent = bytes;
        absent[12..16].copy_from_slice(&0u32.to_ne_bytes());
        let absent = scope_observation(facts, &value, &absent).unwrap();
        assert_eq!(absent.mount, MountFact::Unknown);
        assert_eq!(
            allow_directory(&absent),
            Err(diagnostic(ScanDiagnosticCode::UnsupportedFilesystem))
        );
        let regular = ReadFacts {
            kind: EntryKind::RegularFile,
            ..facts
        };
        assert_eq!(
            scope_observation(regular, &value, &[]).unwrap().cloud,
            CloudFact::Unknown
        );
    }

    // Resource exhaustion must stop the session, not become a filesystem exclusion.
    #[test]
    fn scan_descriptor_exhaustion_stops_root_and_descendant_operations() {
        for errno in [libc::EMFILE, libc::ENFILE] {
            for root in [true, false] {
                assert_eq!(
                    io_error(io::Error::from_raw_os_error(errno), root),
                    ScanIoError::Stopped(StopReason::DescriptorLimit),
                    "errno {errno}, root {root}"
                );
            }
        }
    }

    // Exact root/entry attribution must survive errno sanitization without raw paths.
    #[test]
    fn scan_root_anchor_error_facts_are_sanitized() {
        for (errno, root_code, entry_code) in [
            (
                libc::ENOENT,
                ScanDiagnosticCode::RootMissing,
                ScanDiagnosticCode::EntryMissing,
            ),
            (
                libc::EACCES,
                ScanDiagnosticCode::RootDenied,
                ScanDiagnosticCode::EntryDenied,
            ),
            (
                libc::EPERM,
                ScanDiagnosticCode::RootDenied,
                ScanDiagnosticCode::EntryDenied,
            ),
            (
                libc::ELOOP,
                ScanDiagnosticCode::RootChanged,
                ScanDiagnosticCode::ChangedDuringScan,
            ),
        ] {
            assert_eq!(
                io_error(io::Error::from_raw_os_error(errno), true),
                diagnostic(root_code)
            );
            assert_eq!(
                io_error(io::Error::from_raw_os_error(errno), false),
                diagnostic(entry_code)
            );
        }
    }

    // An after-call stop must close even the just-opened fd / fdopendir stream.
    #[test]
    fn scan_root_anchor_cancellation_checks_each_call_and_releases_descriptors() {
        let temp = tempfile::tempdir().unwrap();
        let stored = saved(temp.path());
        take_events();
        let cursor = filesystem().anchor_root(&stored, &Running).unwrap();
        let expected = cursor.identity();
        drop(cursor);
        let calls = take_events()
            .iter()
            .filter(|e| matches!(e, Event::Call))
            .count();
        assert!(calls > 10);
        for at in 1..=calls * 2 {
            let control = StopAt {
                at,
                calls: std::sync::atomic::AtomicUsize::new(0),
            };
            assert!(
                matches!(
                    filesystem().anchor_root(&stored, &control),
                    Err(ScanIoError::Stopped(StopReason::Cancelled))
                ),
                "checkpoint {at}"
            );
            let events = take_events();
            assert_eq!(
                events.iter().filter(|e| matches!(e, Event::Call)).count(),
                at / 2
            );
            assert!(!events.iter().any(|e| matches!(e, Event::Enumerate(_))));
            for event in events {
                if let Event::Open { fd, .. } = event {
                    assert_released(fd, expected);
                }
            }
        }
    }

    struct HomeIdentity(ObjectIdentity);
    impl AccountHome for HomeIdentity {
        fn identity(&self, _: &dyn ScanIoControl) -> ScanResult<ObjectIdentity> {
            Ok(self.0)
        }
    }
    // Home equality uses native identity, even when the selected path is a symlink alias.
    #[test]
    fn scan_root_anchor_rejects_injected_home_identity_before_enumeration() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("synthetic-home")).unwrap();
        symlink(
            temp.path().join("synthetic-home"),
            temp.path().join("alias"),
        )
        .unwrap();
        let root = saved(&temp.path().join("alias"));
        let cursor = filesystem().anchor_root(&root, &Running).unwrap();
        let fs = NativeFilesystem {
            home: HomeIdentity(cursor.identity()),
        };
        drop(cursor);
        take_events();
        assert!(matches!(
            fs.anchor_root(&root, &Running),
            Err(ScanIoError::Diagnostic(ScanDiagnosticCode::RootTooBroad))
        ));
        assert!(!take_events()
            .iter()
            .any(|e| matches!(e, Event::Enumerate(_))));
    }

    // Injected platform facts: a reported mount root or unsupported/cloud scope cannot enumerate.
    #[test]
    fn scan_root_anchor_injected_mount_and_scope_reject_before_enumeration() {
        let temp = tempfile::tempdir().unwrap();
        let stored = saved(temp.path());
        let cases: [(ScopePatch, ScanDiagnosticCode); 3] = [
            (
                |b| b[52..56].copy_from_slice(&1u32.to_ne_bytes()),
                ScanDiagnosticCode::RootTooBroad,
            ),
            (
                |b| b[12..16].copy_from_slice(&0u32.to_ne_bytes()),
                ScanDiagnosticCode::UnsupportedFilesystem,
            ),
            (
                |b| b[56..64].copy_from_slice(&4u64.to_ne_bytes()),
                ScanDiagnosticCode::CloudScopeUnsupported,
            ),
        ];
        for (patch, code) in cases {
            SCOPE_PATCH.with(|slot| *slot.borrow_mut() = Some(patch));
            take_events();
            assert!(
                matches!(filesystem().anchor_root(&stored, &Running), Err(ScanIoError::Diagnostic(actual)) if actual == code)
            );
            let events = take_events();
            assert!(!events.iter().any(|e| matches!(e, Event::Enumerate(_))));
            assert!(events
                .iter()
                .all(|e| !matches!(e, Event::Open { flags, .. } if flags & libc::O_EVTONLY == 0)));
            SCOPE_PATCH.with(|slot| assert!(slot.borrow().is_none()));
        }
    }

    struct AtCheckpoint {
        at: usize,
        count: std::sync::atomic::AtomicUsize,
        action: std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>>,
    }
    impl ScanIoControl for AtCheckpoint {
        fn checkpoint(&self) -> Result<(), StopReason> {
            if self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1 == self.at {
                self.action.lock().unwrap().take().unwrap()();
            }
            Ok(())
        }
    }

    // Mutation at the openat boundary catches substitution after the last parent check.
    #[test]
    fn scan_cursor_barrier_substitution_cannot_follow_new_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("child")).unwrap();
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("external-sentinel"), b"unchanged").unwrap();
        let mut parent = filesystem().anchor_root(&saved(&root), &Running).unwrap();
        let entry = parent.next_entry(&Running).unwrap().unwrap();
        let child = root.join("child");
        let moved = temp.path().join("moved");
        let control = AtCheckpoint {
            at: 7,
            count: std::sync::atomic::AtomicUsize::new(0),
            action: std::sync::Mutex::new(Some(Box::new(move || {
                fs::rename(&child, &moved).unwrap();
                symlink(&outside, &child).unwrap();
            }))),
        };
        take_events();
        assert!(matches!(
            parent.open_child(&entry, &control),
            Err(ScanIoError::Diagnostic(
                ScanDiagnosticCode::ChangedDuringScan
            ))
        ));
        let events = take_events();
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::OpenAt { .. }))
                .count(),
            1
        );
        assert!(!events.iter().any(|e| matches!(e, Event::Enumerate(_))));
        assert!(control.action.lock().unwrap().is_none());
    }

    // A root replaced after metadata anchoring is a root diagnostic, never a descendant error.
    #[test]
    fn scan_root_anchor_barrier_replacement_is_root_changed() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let stored = saved(&root);
        let moved = temp.path().join("moved");
        let control = AtCheckpoint {
            at: 19,
            count: std::sync::atomic::AtomicUsize::new(0),
            action: std::sync::Mutex::new(Some(Box::new(move || {
                fs::rename(&root, &moved).unwrap();
                fs::create_dir(&root).unwrap();
            }))),
        };
        take_events();
        assert!(matches!(
            filesystem().anchor_root(&stored, &control),
            Err(ScanIoError::Diagnostic(ScanDiagnosticCode::RootChanged))
        ));
        assert!(control.action.lock().unwrap().is_none());
        assert!(!take_events()
            .iter()
            .any(|e| matches!(e, Event::Enumerate(_))));
    }

    // Retaining each parent must use one descriptor per depth, never a parent dup.
    #[test]
    fn scan_cursor_depth_32_owns_33_directory_descriptors() {
        let temp = tempfile::tempdir().unwrap();
        let mut path = temp.path().to_path_buf();
        for _ in 0..32 {
            path.push("child");
            fs::create_dir(&path).unwrap();
        }
        let root = filesystem()
            .anchor_root(&saved(temp.path()), &Running)
            .unwrap();
        take_events();
        let mut stack = vec![root];
        for _ in 0..32 {
            let parent = stack.last_mut().unwrap();
            let entry = parent.next_entry(&Running).unwrap().unwrap();
            let child = parent.open_child(&entry, &Running).unwrap();
            parent
                .verify_link(&entry, &child.identity(), &Running)
                .unwrap();
            stack.push(child);
        }
        let events = take_events();
        let fds: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                Event::Open { fd, .. } => Some(*fd),
                _ => None,
            })
            .collect();
        assert_eq!(fds.len(), 32);
        assert_eq!(
            fds.iter().collect::<std::collections::HashSet<_>>().len(),
            32
        );
        let identities: Vec<_> = stack
            .iter()
            .skip(1)
            .map(|cursor| cursor.identity())
            .collect();
        drop(stack);
        assert_eq!(
            take_events()
                .iter()
                .filter(|event| matches!(event, Event::Closed(_)))
                .count(),
            33
        );
        for (fd, identity) in fds.into_iter().zip(identities) {
            assert_released(fd, identity);
        }
    }

    // An invalid UTF-8 dirent is still an exact native component, not a lossy IPC path.
    #[test]
    fn scan_metadata_injected_native_bytes_survive_without_path_conversion() {
        // APFS rejected creating this real filename with EPERM; use a complete native dirent fixture.
        let mut entry: libc::dirent = unsafe { std::mem::zeroed() };
        entry.d_namlen = 3;
        entry.d_name[0] = b'a' as libc::c_char;
        entry.d_name[1] = 0xffu8 as libc::c_char;
        entry.d_name[2] = b'z' as libc::c_char;
        let name = native_name(&entry).unwrap();
        assert_eq!(name.as_bytes(), &[b'a', 0xff, b'z']);
        assert!(name.to_str().is_none());
        assert_eq!(component(&name).unwrap().as_bytes(), &[b'a', 0xff, b'z']);
        for name in [b"".as_slice(), b".", b"..", b"a/b", b"a\0b"] {
            assert!(component(OsStr::from_bytes(name)).is_err());
        }
        entry.d_namlen = u16::MAX;
        assert!(native_name(&entry).is_err());
    }

    // Every cursor operation must stop inside its native calls, not only at entry.
    #[test]
    fn scan_cursor_cancellation_checks_each_call_in_all_operations() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("child")).unwrap();
        let stored = saved(temp.path());
        let exercise = |operation: usize, control: &dyn ScanIoControl| {
            let mut parent = filesystem().anchor_root(&stored, &Running).unwrap();
            let entry = parent.next_entry(&Running).unwrap().unwrap();
            let child = parent.open_child(&entry, &Running).unwrap();
            let expected = child.identity();
            // next_entry needs its own fresh cursor, since the setup consumed the entry.
            let mut next_cursor = filesystem().anchor_root(&stored, &Running).unwrap();
            take_events();
            let result = match operation {
                0 => next_cursor.next_entry(control).map(|_| ()),
                1 => parent.open_child(&entry, control).map(drop),
                2 => parent.verify_link(&entry, &child.identity(), control),
                _ => parent.finish(control),
            };
            drop(next_cursor);
            drop(child);
            drop(parent);
            (result, take_events(), expected)
        };
        for operation in 0..4 {
            let (result, events, _) = exercise(operation, &Running);
            result.unwrap();
            let calls = events.iter().filter(|e| matches!(e, Event::Call)).count();
            for at in 1..=calls * 2 {
                let control = StopAt {
                    at,
                    calls: std::sync::atomic::AtomicUsize::new(0),
                };
                let (result, events, expected) = exercise(operation, &control);
                assert_eq!(
                    result,
                    Err(ScanIoError::Stopped(StopReason::Cancelled)),
                    "operation {operation} checkpoint {at}"
                );
                assert_eq!(
                    events.iter().filter(|e| matches!(e, Event::Call)).count(),
                    at / 2
                );
                for event in events {
                    if let Event::Open { fd, .. } = event {
                        assert_released(fd, expected);
                    }
                }
            }
        }
    }
}
