//! Validation for the fixed boot-image slots.

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Truncated,
    Bounds,
    Header,
    Compressed,
}

fn word(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data.get(offset..offset + 4).ok_or(Error::Truncated)?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

pub fn uboot_length(header: &[u8], capacity: usize) -> Result<usize, Error> {
    if word(header, 0)? != 0x5000_0000 {
        return Err(Error::Header);
    }
    let len = word(header, 4)? as usize;
    if len < 4 || len > capacity {
        return Err(Error::Bounds);
    }
    Ok(len)
}

pub fn validate_uboot(data: &[u8]) -> Result<(), Error> {
    match word(data, 0)? {
        0x184d_2204 | 0x184c_2102 => Err(Error::Compressed),
        _ if data.starts_with(&[0x1f, 0x8b]) => Err(Error::Compressed),
        0 | u32::MAX => Err(Error::Header),
        _ => Ok(()),
    }
}

pub fn opensbi_length(header: &[u8], capacity: usize) -> Result<usize, Error> {
    if word(header, 4)? != 0x4942_534f || word(header, 8)? != 1 || word(header, 12)? != 64 {
        return Err(Error::Header);
    }
    let len = word(header, 16)? as usize;
    // Flags must not override the loader's boot arguments.
    if word(header, 20)? != 0 || len < 128 || len > capacity {
        return Err(Error::Bounds);
    }
    Ok(len)
}

pub fn fdt_length(header: &[u8], capacity: usize) -> Result<usize, Error> {
    if header.len() < 40 {
        return Err(Error::Truncated);
    }
    let be = |i| u32::from_be_bytes(header[i..i + 4].try_into().unwrap()) as usize;
    if be(0) != 0xd00d_feed || be(20) < 17 || be(24) > 17 {
        return Err(Error::Header);
    }
    let len = be(4);
    if len < 40 || len > capacity {
        return Err(Error::Bounds);
    }
    for (offset, size) in [(be(8), be(36)), (be(12), be(32)), (be(16), 16)] {
        if offset < 40 || offset.checked_add(size).is_none_or(|end| end > len) {
            return Err(Error::Bounds);
        }
    }
    Ok(len)
}
