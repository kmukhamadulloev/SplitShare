//! Single byte ranges; multiple ranges are intentionally unsupported.
#[derive(Debug, PartialEq)]
pub struct ByteRange {
    pub start: u64,
    pub length: u64,
}
pub fn parse(value: &str, size: u64) -> Result<ByteRange, ()> {
    let value = value.trim().strip_prefix("bytes=").ok_or(())?;
    let (start, end) = value.split_once('-').ok_or(())?;
    fn number(value: &str) -> Result<u64, ()> {
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(());
        }
        value.parse().map_err(|_| ())
    }
    if size == 0 {
        return Err(());
    }
    if start.is_empty() {
        let suffix = number(end)?;
        if suffix == 0 {
            return Err(());
        }
        let length = suffix.min(size);
        return Ok(ByteRange {
            start: size - length,
            length,
        });
    }
    let start = number(start)?;
    if start >= size {
        return Err(());
    }
    let end = if end.is_empty() {
        size - 1
    } else {
        number(end)?.min(size - 1)
    };
    if end < start {
        return Err(());
    }
    Ok(ByteRange {
        start,
        length: end - start + 1,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supported_ranges_and_invalid_inputs() {
        for (header, start, length) in [
            ("bytes=0-2", 0, 3),
            ("bytes=3-", 3, 7),
            ("bytes=-3", 7, 3),
            ("bytes=0-999", 0, 10),
            ("bytes=-999", 0, 10),
        ] {
            assert_eq!(parse(header, 10), Ok(ByteRange { start, length }));
        }
        for header in [
            "bytes=",
            "bytes=-",
            "bytes=-0",
            "bytes=10-",
            "bytes=5-4",
            "bytes=0-1,3-4",
            "items=0-1",
            "bytes=+1-3",
            "bytes=0-18446744073709551616",
            "bytes=0--1",
        ] {
            assert!(parse(header, 10).is_err(), "{header}");
        }
        assert!(parse("bytes=0-", 0).is_err());
    }
}
