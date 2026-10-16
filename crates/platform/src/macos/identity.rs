//! Checked decoding shared with the saved macos-v1 root identity.
#[derive(Debug)]
pub(super) struct IdentityError;

pub(super) fn wire_identity(value: dev_storage_core::scanner::ObjectIdentity) -> String {
    let volume: String = value
        .volume_uuid
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!(
        "macos-v1:{volume}:{:x}:{}:{}",
        value.file_id, value.birth_seconds, value.birth_nanoseconds
    )
}

pub(super) const ROOT_ATTRIBUTES: u32 = libc::ATTR_CMN_RETURNED_ATTRS
    | libc::ATTR_CMN_OBJTYPE
    | libc::ATTR_CMN_CRTIME
    | libc::ATTR_CMN_FILEID;
pub(super) const VOLUME_ATTRIBUTES: u32 =
    libc::ATTR_VOL_INFO | libc::ATTR_VOL_CAPABILITIES | libc::ATTR_VOL_UUID;

pub(super) fn array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], IdentityError> {
    bytes
        .get(offset..offset.checked_add(N).ok_or(IdentityError)?)
        .ok_or(IdentityError)?
        .try_into()
        .map_err(|_| IdentityError)
}

pub(super) fn attribute_payload(
    bytes: &[u8],
    length: usize,
    common: u32,
    volume: u32,
) -> Result<&[u8], IdentityError> {
    let actual_length = u32::from_ne_bytes(array(bytes, 0)?) as usize;
    if actual_length != length || actual_length > bytes.len() {
        return Err(IdentityError);
    }
    let bytes = &bytes[..actual_length];
    // Require every data-bearing attribute and reject unexpected data fields;
    // ATTR_VOL_INFO is a request marker with no bytes, so its return bit is optional.
    if u32::from_ne_bytes(array(bytes, 4)?) != common
        || (u32::from_ne_bytes(array(bytes, 8)?) & !libc::ATTR_VOL_INFO) != volume
        || bytes[12..24] != [0; 12]
    {
        return Err(IdentityError);
    }
    Ok(bytes)
}

pub(super) fn decode_root(bytes: &[u8]) -> Result<(bool, u64, i64, i64), IdentityError> {
    let bytes = attribute_payload(bytes, 52, ROOT_ATTRIBUTES, 0)?;
    // Darwin enum vtype VDIR = 2; fsobj_type_t is a 32-bit native integer.
    let is_directory = u32::from_ne_bytes(array(bytes, 24)?) == 2;
    let seconds = i64::from_ne_bytes(array(bytes, 28)?);
    let nanoseconds = i64::from_ne_bytes(array(bytes, 36)?);
    let file_id = u64::from_ne_bytes(array(bytes, 44)?);
    Ok((is_directory, file_id, seconds, nanoseconds))
}

pub(super) fn decode_volume(bytes: &[u8]) -> Result<([u8; 16], bool), IdentityError> {
    let bytes = attribute_payload(
        bytes,
        72,
        libc::ATTR_CMN_RETURNED_ATTRS,
        libc::ATTR_VOL_CAPABILITIES | libc::ATTR_VOL_UUID,
    )?;
    let capabilities = u32::from_ne_bytes(array(bytes, 24)?);
    let valid = u32::from_ne_bytes(array(bytes, 40)?);
    let persistent = (capabilities & valid & libc::VOL_CAP_FMT_PERSISTENTOBJECTIDS) != 0;
    Ok((array(bytes, 56)?, persistent))
}
