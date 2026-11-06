//! Fallible container keys and record storage; payload limits remain separate.
use super::{DataMap, DecodeLimits, Error, add, limits, mul};
const MAX_RECORDS: usize = 2 * super::MAX_AUX_BOX_INDEX; // at most two records per admitted auxiliary box
pub(super) fn record_bytes(capacity: usize) -> Result<usize, Error> {
    mul(capacity, std::mem::size_of::<(String, DataMap)>())
}
pub(super) fn workspace(base: usize, capacity: usize, finite: bool) -> Result<usize, Error> {
    if finite {
        add(base, record_bytes(capacity)?)
    } else {
        Ok(base)
    }
}
pub(super) fn key_len(stage: &str, index: usize, kind: &[u8]) -> Result<usize, Error> {
    let digits = if index == 0 {
        1
    } else {
        index.ilog10() as usize + 1
    };
    add(
        add(add(8, stage.len())?, add(6, digits)?)?,
        mul(kind.len(), 2)?,
    )
}
pub(super) fn key(stage: &str, index: usize, kind: &[u8]) -> Result<String, Error> {
    let length = key_len(stage, index, kind)?;
    let mut key = String::new();
    #[cfg(test)]
    super::resources::tests::before_metadata_copy(stage, length);
    key.try_reserve_exact(length)?;
    key.push_str("JPEG XL ");
    key.push_str(stage);
    key.push_str(" Box ");
    let mut digits = [0u8; usize::BITS as usize];
    let mut at = digits.len();
    let mut n = index;
    loop {
        at -= 1;
        digits[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    for &b in &digits[at..] {
        key.push(b as char)
    }
    key.push(' ');
    for &b in kind {
        key.push(b"0123456789abcdef"[(b >> 4) as usize] as char);
        key.push(b"0123456789abcdef"[(b & 15) as usize] as char)
    }
    debug_assert_eq!(key.len(), length);
    Ok(key)
}
/// Account old+replacement backing storage before a possible realloc. A failed
/// try_reserve_exact preserves the old Vec and all its payload owners.
pub(super) fn reserve(
    metadata: &mut Vec<(String, DataMap)>,
    additional: usize,
    retained: usize,
    input_bytes: usize,
    limits: DecodeLimits,
    finite: bool,
) -> Result<(), Error> {
    let required = add(metadata.len(), additional)?;
    limits::check(
        required,
        MAX_RECORDS,
        "JPEG XL container metadata record count",
    )?;
    let grow = required > metadata.capacity();
    let capacity = if grow {
        required
            .max(mul(metadata.capacity(), 2)?)
            .clamp(4, MAX_RECORDS)
    } else {
        metadata.capacity()
    };
    if finite {
        let base = add(mul(input_bytes, 3)?, mul(retained, 2)?)?;
        let peak = add(
            record_bytes(metadata.capacity())?,
            if grow { record_bytes(capacity)? } else { 0 },
        )?;
        limits::check(
            add(base, peak)?,
            limits.expanded_bytes,
            "JPEG XL container workspace",
        )?;
    }
    if grow {
        #[cfg(test)]
        super::resources::tests::before_metadata_copy("container records", record_bytes(capacity)?);
        metadata.try_reserve_exact(capacity - metadata.len())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_match_legacy_schema_for_decimal_boundaries_and_binary_types() {
        for stage in ["Source", "Decoded"] {
            for index in [0, 9, 10, 99, 100, 1024, usize::MAX] {
                let expected = format!("JPEG XL {stage} Box {index} 002080ff");
                let actual = key(stage, index, &[0, 0x20, 0x80, 0xff]).unwrap();
                assert_eq!(actual, expected);
                assert_eq!(
                    actual.len(),
                    key_len(stage, index, &[0, 0x20, 0x80, 0xff]).unwrap()
                );
            }
        }
    }
}
