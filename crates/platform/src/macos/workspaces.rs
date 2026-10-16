use super::identity::{ROOT_ATTRIBUTES, VOLUME_ATTRIBUTES};
use dev_storage_core::workspaces::{
    AvailabilityObservation, AvailabilityReason, AvailabilityState, DirectoryIdentity,
    ValidatedWorkspace, WorkspaceErrorCode, WorkspaceFailure,
};
use std::{
    ffi::CString,
    io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
struct RootMetadata {
    canonical_path: PathBuf,
    is_directory: bool,
    volume_uuid: [u8; 16],
    persistent_ids: bool,
    file_id: u64,
    birth_seconds: i64,
    birth_nanoseconds: i64,
}

#[derive(Debug)]
enum RootError {
    Io(io::Error),
    Code(WorkspaceErrorCode),
}

impl RootError {
    fn from_io(error: io::Error) -> Self {
        let code = match error.raw_os_error() {
            Some(libc::ENOENT) => WorkspaceErrorCode::FolderMissing,
            Some(libc::ENOTDIR) => WorkspaceErrorCode::NotDirectory,
            Some(libc::EACCES | libc::EPERM) => WorkspaceErrorCode::AccessDenied,
            Some(libc::ELOOP) => WorkspaceErrorCode::InvalidSelection,
            Some(libc::ENOTSUP | libc::ENOSYS) => WorkspaceErrorCode::IdentityUnavailable,
            _ => return Self::Io(error),
        };
        Self::Code(code)
    }

    fn normalized(self) -> Self {
        match self {
            Self::Io(error) => Self::from_io(error),
            other => other,
        }
    }

    fn failure(self) -> WorkspaceFailure {
        let code = match self.normalized() {
            Self::Code(code) => code,
            Self::Io(_) => WorkspaceErrorCode::IdentityUnavailable,
        };
        WorkspaceFailure {
            code,
            existing_id: None,
        }
    }
}

trait MetadataReader {
    fn read_root(&self, path: &Path) -> Result<RootMetadata, RootError>;
}

struct NativeMetadataReader;
impl MetadataReader for NativeMetadataReader {
    fn read_root(&self, path: &Path) -> Result<RootMetadata, RootError> {
        let name = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| RootError::Code(WorkspaceErrorCode::InvalidSelection))?;
        // O_EVTONLY obtains a metadata-only reference: no read/write access to contents.
        // SAFETY: name is NUL-terminated; no mode argument is needed without O_CREAT.
        let raw_fd = unsafe { libc::open(name.as_ptr(), libc::O_EVTONLY | libc::O_CLOEXEC) };
        if raw_fd < 0 {
            return Err(RootError::from_io(io::Error::last_os_error()));
        }
        // SAFETY: open returned a new owned descriptor, closed by OwnedFd on all paths.
        let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };
        let (is_directory, file_id, birth_seconds, birth_nanoseconds) =
            decode_root(&read_attributes(&fd, ROOT_ATTRIBUTES, 0)?)?;
        if !is_directory {
            return Err(RootError::Code(WorkspaceErrorCode::NotDirectory));
        }
        // Volume requests resolve the volume root inside macOS. Keep them separate
        // from object attributes so file ID and birth time belong to this directory.
        let (volume_uuid, persistent_ids) = decode_volume(&read_attributes(
            &fd,
            libc::ATTR_CMN_RETURNED_ATTRS,
            VOLUME_ATTRIBUTES,
        )?)?;
        let canonical_path = path.canonicalize().map_err(RootError::from_io)?;
        Ok(RootMetadata {
            canonical_path,
            is_directory,
            volume_uuid,
            persistent_ids,
            file_id,
            birth_seconds,
            birth_nanoseconds,
        })
    }
}

pub(super) fn validate(selected: &Path) -> Result<ValidatedWorkspace, WorkspaceFailure> {
    validate_with(&NativeMetadataReader, selected).map_err(RootError::failure)
}

pub(super) fn check(root: &ValidatedWorkspace, now: i64) -> AvailabilityObservation {
    check_with(&NativeMetadataReader, root, now)
}

fn identity(metadata: &RootMetadata) -> Result<DirectoryIdentity, RootError> {
    if !metadata.is_directory {
        return Err(RootError::Code(WorkspaceErrorCode::NotDirectory));
    }
    if metadata.canonical_path.to_str().is_none() {
        return Err(RootError::Code(WorkspaceErrorCode::UnsupportedPathEncoding));
    }
    if !metadata.canonical_path.is_absolute()
        || !metadata.persistent_ids
        || metadata.volume_uuid == [0; 16]
        || metadata.file_id == 0
        || !(0..1_000_000_000).contains(&metadata.birth_nanoseconds)
    {
        return Err(RootError::Code(WorkspaceErrorCode::IdentityUnavailable));
    }
    Ok(DirectoryIdentity::new(super::identity::wire_identity(
        dev_storage_core::scanner::ObjectIdentity {
            volume_uuid: metadata.volume_uuid,
            file_id: metadata.file_id,
            birth_seconds: metadata.birth_seconds,
            birth_nanoseconds: metadata.birth_nanoseconds,
        },
    )))
}

fn validate_with(
    reader: &impl MetadataReader,
    selected: &Path,
) -> Result<ValidatedWorkspace, RootError> {
    let selected_path = selected
        .to_str()
        .ok_or(RootError::Code(WorkspaceErrorCode::UnsupportedPathEncoding))?;
    if !selected.is_absolute() || selected_path.as_bytes().contains(&0) {
        return Err(RootError::Code(WorkspaceErrorCode::InvalidSelection));
    }
    let first = reader.read_root(selected).map_err(RootError::normalized)?;
    let expected = identity(&first)?;
    // Two observations of each reference reject observed symlink retargeting,
    // directory replacement, and canonical-path changes. This is not a filesystem
    // transaction and never constitutes authority for a later destructive action.
    for reference in [
        first.canonical_path.as_path(),
        selected,
        first.canonical_path.as_path(),
    ] {
        let observed = reader.read_root(reference).map_err(RootError::normalized)?;
        if identity(&observed)? != expected || observed.canonical_path != first.canonical_path {
            return Err(RootError::Code(WorkspaceErrorCode::LocationChanged));
        }
    }
    Ok(ValidatedWorkspace {
        selected_path: selected_path.into(),
        canonical_path: first
            .canonical_path
            .to_str()
            .expect("identity checked UTF-8")
            .into(),
        identity: expected,
    })
}

fn check_with(
    reader: &impl MetadataReader,
    root: &ValidatedWorkspace,
    now: i64,
) -> AvailabilityObservation {
    let result = validate_with(reader, Path::new(&root.selected_path));
    let (state, reason) = match result {
        Ok(observed)
            if observed.identity == root.identity
                && observed.canonical_path == root.canonical_path =>
        {
            (AvailabilityState::Present, None)
        }
        Ok(_) | Err(RootError::Code(WorkspaceErrorCode::LocationChanged)) => (
            AvailabilityState::Unknown,
            Some(AvailabilityReason::IdentityChanged),
        ),
        Err(RootError::Code(WorkspaceErrorCode::FolderMissing)) => {
            (AvailabilityState::Missing, None)
        }
        Err(RootError::Code(WorkspaceErrorCode::AccessDenied)) => (AvailabilityState::Denied, None),
        Err(RootError::Code(WorkspaceErrorCode::NotDirectory)) => (
            AvailabilityState::Unknown,
            Some(AvailabilityReason::NotDirectory),
        ),
        Err(RootError::Code(WorkspaceErrorCode::IdentityUnavailable)) => (
            AvailabilityState::Unknown,
            Some(AvailabilityReason::UnsupportedIdentity),
        ),
        Err(_) => (AvailabilityState::Unknown, Some(AvailabilityReason::Io)),
    };
    AvailabilityObservation {
        state,
        reason,
        checked_at: Some(now),
    }
}

fn read_attributes(fd: &OwnedFd, commonattr: u32, volattr: u32) -> Result<[u8; 128], RootError> {
    let mut attributes = libc::attrlist {
        bitmapcount: libc::ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr,
        volattr,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };
    let mut bytes = [0u8; 128];
    // SAFETY: both pointers are valid for their complete declared sizes. The
    // syscall writes bytes only. Parsing below uses checked slices, never typed
    // pointer dereferences (the ABI packs attributes at 4-byte alignment).
    let result = unsafe {
        libc::fgetattrlist(
            fd.as_raw_fd(),
            (&mut attributes as *mut libc::attrlist).cast(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            libc::FSOPT_REPORT_FULLSIZE,
        )
    };
    if result != 0 {
        return Err(RootError::from_io(io::Error::last_os_error()));
    }
    Ok(bytes)
}

fn unavailable() -> RootError {
    RootError::Code(WorkspaceErrorCode::IdentityUnavailable)
}

fn decode_root(bytes: &[u8]) -> Result<(bool, u64, i64, i64), RootError> {
    super::identity::decode_root(bytes).map_err(|_| unavailable())
}

fn decode_volume(bytes: &[u8]) -> Result<([u8; 16], bool), RootError> {
    super::identity::decode_volume(bytes).map_err(|_| unavailable())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::RefCell,
        collections::VecDeque,
        ffi::OsString,
        fs,
        os::unix::{
            ffi::OsStringExt,
            fs::{symlink, PermissionsExt},
        },
    };

    // Every call is enclosed by a sentinel check: production must never touch contents.
    struct Fixture {
        temp: tempfile::TempDir,
        sentinel: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let sentinel = temp.path().join("sentinel");
            fs::write(&sentinel, b"unchanged\0contents\xff").unwrap();
            Self { temp, sentinel }
        }
        fn around<T>(&self, call: impl FnOnce() -> T) -> T {
            let before = fs::read(&self.sentinel).unwrap();
            let result = call();
            assert_eq!(fs::read(&self.sentinel).unwrap(), before);
            result
        }
        fn validate(&self, path: &Path) -> Result<ValidatedWorkspace, RootError> {
            self.around(|| validate_with(&NativeMetadataReader, path))
        }
        fn check(&self, root: &ValidatedWorkspace) -> AvailabilityObservation {
            self.around(|| check_with(&NativeMetadataReader, root, 1234))
        }
    }

    fn code(error: RootError) -> WorkspaceErrorCode {
        match error {
            RootError::Code(code) => code,
            RootError::Io(_) => panic!("unmapped IO error"),
        }
    }

    fn metadata(path: &str, file_id: u64) -> RootMetadata {
        RootMetadata {
            canonical_path: path.into(),
            is_directory: true,
            volume_uuid: [1; 16],
            persistent_ids: true,
            file_id,
            birth_seconds: 123,
            birth_nanoseconds: 456,
        }
    }

    // The seam only offers root metadata. No content, enumeration or mutation API exists.
    struct ScriptedReader {
        replies: RefCell<VecDeque<Result<RootMetadata, RootError>>>,
        requests: RefCell<Vec<PathBuf>>,
    }
    impl ScriptedReader {
        fn new(replies: Vec<Result<RootMetadata, RootError>>) -> Self {
            Self {
                replies: RefCell::new(replies.into()),
                requests: RefCell::new(vec![]),
            }
        }
        fn stable(value: RootMetadata) -> Self {
            Self::new((0..4).map(|_| Ok(value.clone())).collect())
        }
    }
    impl MetadataReader for ScriptedReader {
        fn read_root(&self, path: &Path) -> Result<RootMetadata, RootError> {
            self.requests.borrow_mut().push(path.into());
            self.replies
                .borrow_mut()
                .pop_front()
                .expect("unexpected metadata request")
        }
    }

    // Catches lossy path conversion, artificial path limits, and unstable native identity.
    #[test]
    fn workspace_root_accepts_unicode_spaces_and_long_paths() {
        let f = Fixture::new();
        let mut path = f.temp.path().to_path_buf();
        for _ in 0..6 {
            path.push("long directory component with spaces");
        }
        path.push("Café 工具 space");
        fs::create_dir_all(&path).unwrap();
        assert!(path.as_os_str().len() > 260);
        let root = f.validate(&path).unwrap();
        assert_eq!(root.selected_path, path.to_str().unwrap());
        assert_eq!(root.identity, f.validate(&path).unwrap().identity);
        assert!(root.identity.as_str().starts_with("macos-v1:"));
    }

    // Catches lexical identity, case-folding, and conflating equal basenames.
    #[test]
    fn workspace_root_aliases_and_case() {
        let f = Fixture::new();
        let root_path = f.temp.path().join("Code");
        let other = f.temp.path().join("other/Code");
        fs::create_dir_all(root_path.join("child")).unwrap();
        fs::create_dir_all(&other).unwrap();
        let alias = f.temp.path().join("alias");
        symlink(&root_path, &alias).unwrap();
        let root = f.validate(&root_path).unwrap();
        for path in [&alias, &root_path.join("./child/..")] {
            assert_eq!(f.validate(path).unwrap().identity, root.identity);
        }
        assert_ne!(f.validate(&other).unwrap().identity, root.identity);
        let case_variant = f.temp.path().join("code");
        if case_variant.try_exists().unwrap() {
            assert_eq!(f.validate(&case_variant).unwrap().identity, root.identity);
            eprintln!("REAL FIXTURE: case-insensitive volume alias verified");
        } else {
            fs::create_dir(&case_variant).unwrap();
            assert_ne!(f.validate(&case_variant).unwrap().identity, root.identity);
            eprintln!("REAL FIXTURE: case-sensitive volume distinct directories verified");
        }
        // Simulated case-sensitive volume, not evidence of this disk's behavior.
        let upper = validate_with(
            &ScriptedReader::stable(metadata("/fixture/Code", 10)),
            Path::new("/fixture/Code"),
        )
        .unwrap();
        let lower = validate_with(
            &ScriptedReader::stable(metadata("/fixture/code", 11)),
            Path::new("/fixture/code"),
        )
        .unwrap();
        assert_ne!(upper.identity, lower.identity);
    }

    // Catches accepting invalid targets, or normalizing unsupported path bytes.
    #[test]
    fn workspace_root_rejects_changed_or_invalid_targets() {
        let f = Fixture::new();
        let missing = f.temp.path().join("missing");
        let dangling = f.temp.path().join("dangling");
        symlink(&missing, &dangling).unwrap();
        let loop_path = f.temp.path().join("loop");
        symlink(&loop_path, &loop_path).unwrap();
        let non_utf8 = f.temp.path().join(OsString::from_vec(vec![b'a', 0xff]));
        for (path, expected) in [
            (f.sentinel.as_path(), WorkspaceErrorCode::NotDirectory),
            (&missing, WorkspaceErrorCode::FolderMissing),
            (&dangling, WorkspaceErrorCode::FolderMissing),
            (&loop_path, WorkspaceErrorCode::InvalidSelection),
            (&non_utf8, WorkspaceErrorCode::UnsupportedPathEncoding),
            (Path::new("relative"), WorkspaceErrorCode::InvalidSelection),
        ] {
            assert_eq!(code(f.validate(path).unwrap_err()), expected);
        }
        for errno in [libc::EACCES, libc::EPERM] {
            let reader = ScriptedReader::new(vec![Err(RootError::Io(
                io::Error::from_raw_os_error(errno),
            ))]);
            assert_eq!(
                code(validate_with(&reader, Path::new("/fixture")).unwrap_err()),
                WorkspaceErrorCode::AccessDenied
            );
        }
        for bad in [
            RootMetadata {
                volume_uuid: [0; 16],
                ..metadata("/fixture", 10)
            },
            RootMetadata {
                persistent_ids: false,
                ..metadata("/fixture", 10)
            },
            RootMetadata {
                file_id: 0,
                ..metadata("/fixture", 10)
            },
            RootMetadata {
                canonical_path: non_utf8,
                ..metadata("/fixture", 10)
            },
        ] {
            let expected = if bad.canonical_path.to_str().is_none() {
                WorkspaceErrorCode::UnsupportedPathEncoding
            } else {
                WorkspaceErrorCode::IdentityUnavailable
            };
            assert_eq!(
                code(
                    validate_with(&ScriptedReader::stable(bad), Path::new("/fixture")).unwrap_err()
                ),
                expected
            );
        }
        for changed_at in 1..4 {
            let reader = ScriptedReader::new(
                (0..4)
                    .map(|i| Ok(metadata("/fixture", if i == changed_at { 99 } else { 10 })))
                    .collect(),
            );
            assert_eq!(
                code(validate_with(&reader, Path::new("/fixture")).unwrap_err()),
                WorkspaceErrorCode::LocationChanged
            );
        }
        let reader = ScriptedReader::new(vec![
            Ok(metadata("/target/a", 10)),
            Ok(metadata("/target/a", 10)),
            Ok(metadata("/target/b", 10)),
        ]);
        assert_eq!(
            code(validate_with(&reader, Path::new("/alias")).unwrap_err()),
            WorkspaceErrorCode::LocationChanged
        );
    }

    // Catches stale success, rebound replacement roots, and checking inaccessible children.
    #[test]
    fn workspace_root_disconnect_and_return() {
        let f = Fixture::new();
        let path = f.temp.path().join("workspace");
        fs::create_dir(&path).unwrap();
        let child = path.join("inaccessible");
        fs::create_dir(&child).unwrap();
        fs::set_permissions(&child, fs::Permissions::from_mode(0o000)).unwrap();
        let root = f.validate(&path).unwrap();
        assert_eq!(
            f.check(&root),
            AvailabilityObservation {
                state: AvailabilityState::Present,
                reason: None,
                checked_at: Some(1234)
            }
        );
        let parked = f.temp.path().join("parked");
        fs::rename(&path, &parked).unwrap();
        assert_eq!(f.check(&root).state, AvailabilityState::Missing);
        fs::rename(&parked, &path).unwrap();
        assert_eq!(f.check(&root).state, AvailabilityState::Present);
        assert_eq!(f.validate(&path).unwrap().identity, root.identity);
        fs::rename(&path, &parked).unwrap();
        fs::create_dir(&path).unwrap();
        let replaced = f.check(&root);
        assert_eq!(replaced.state, AvailabilityState::Unknown);
        assert_eq!(replaced.reason, Some(AvailabilityReason::IdentityChanged));
        fs::set_permissions(
            parked.join("inaccessible"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
    }

    // Catches wrong error states and reads outside selected/canonical references.
    #[test]
    fn workspace_root_metadata_requests_and_availability_errors() {
        let reader = ScriptedReader::stable(metadata("/canonical", 10));
        let root = validate_with(&reader, Path::new("/selected")).unwrap();
        assert_eq!(
            *reader.requests.borrow(),
            ["/selected", "/canonical", "/selected", "/canonical"].map(PathBuf::from)
        );
        assert_eq!(
            root.identity.as_str(),
            "macos-v1:01010101010101010101010101010101:a:123:456"
        );
        for (errno, state, reason) in [
            (libc::ENOENT, AvailabilityState::Missing, None),
            (libc::EACCES, AvailabilityState::Denied, None),
            (libc::EPERM, AvailabilityState::Denied, None),
            (
                libc::EIO,
                AvailabilityState::Unknown,
                Some(AvailabilityReason::Io),
            ),
            (
                libc::ENOTSUP,
                AvailabilityState::Unknown,
                Some(AvailabilityReason::UnsupportedIdentity),
            ),
        ] {
            let reader = ScriptedReader::new(vec![Err(RootError::Io(
                io::Error::from_raw_os_error(errno),
            ))]);
            assert_eq!(
                check_with(&reader, &root, 42),
                AvailabilityObservation {
                    state,
                    reason,
                    checked_at: Some(42)
                }
            );
        }
        for (value, reason) in [
            (
                RootMetadata {
                    is_directory: false,
                    ..metadata("/canonical", 10)
                },
                AvailabilityReason::NotDirectory,
            ),
            (
                RootMetadata {
                    volume_uuid: [0; 16],
                    ..metadata("/canonical", 10)
                },
                AvailabilityReason::UnsupportedIdentity,
            ),
        ] {
            let observation = check_with(&ScriptedReader::stable(value), &root, 42);
            assert_eq!(observation.state, AvailabilityState::Unknown);
            assert_eq!(observation.reason, Some(reason));
        }
    }

    fn root_buffer() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [52u32, 0x82000208, 0, 0, 0, 0, 2] {
            bytes.extend(value.to_ne_bytes());
        }
        bytes.extend(123i64.to_ne_bytes());
        bytes.extend(456i64.to_ne_bytes());
        bytes.extend(10u64.to_ne_bytes());
        bytes
    }
    fn volume_buffer() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [
            72u32, 0x80000000, 0x80060000, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0,
        ] {
            bytes.extend(value.to_ne_bytes());
        }
        bytes.extend([1; 16]);
        bytes
    }

    // Catches out-of-bounds/unaligned decoding and trusting missing attribute bits.
    #[test]
    fn workspace_root_attribute_buffers_require_lengths_and_returned_masks() {
        let root = root_buffer();
        let volume = volume_buffer();
        assert_eq!(decode_root(&root).unwrap(), (true, 10, 123, 456));
        assert_eq!(decode_volume(&volume).unwrap(), ([1; 16], true));
        for length in 0..root.len() {
            assert_eq!(
                code(decode_root(&root[..length]).unwrap_err()),
                WorkspaceErrorCode::IdentityUnavailable
            );
        }
        for length in 0..volume.len() {
            assert_eq!(
                code(decode_volume(&volume[..length]).unwrap_err()),
                WorkspaceErrorCode::IdentityUnavailable
            );
        }
        for bit in [0x80000000u32, 0x8, 0x200, 0x2000000] {
            let mut bad = root.clone();
            bad[4..8].copy_from_slice(&(0x82000208u32 & !bit).to_ne_bytes());
            assert!(decode_root(&bad).is_err());
        }
        for bit in [0x20000u32, 0x40000] {
            let mut bad = volume.clone();
            bad[8..12].copy_from_slice(&(0x80060000u32 & !bit).to_ne_bytes());
            assert!(decode_volume(&bad).is_err());
        }
        for length in [0u32, 24, 51, 53, u32::MAX] {
            let mut bad = root.clone();
            bad[..4].copy_from_slice(&length.to_ne_bytes());
            assert!(decode_root(&bad).is_err());
        }
        let mut unaligned = vec![0];
        unaligned.extend(&root);
        assert_eq!(decode_root(&unaligned[1..]).unwrap(), (true, 10, 123, 456));
        let mut unsupported = volume.clone();
        unsupported[40..44].copy_from_slice(&0u32.to_ne_bytes());
        assert_eq!(decode_volume(&unsupported).unwrap(), ([1; 16], false));
    }
}
