use chrono::{DateTime, Utc};

use nom::{
    bytes::complete::take, combinator::map, error::ParseError, number::complete::le_i64, IResult,
    Parser as _,
};

#[inline]
pub fn timestamp(input: &[u8]) -> IResult<&[u8], DateTime<Utc>> {
    let (input, unix_time) = le_i64.parse(input)?;
    Ok((input, DateTime::from_timestamp_nanos(unix_time)))
}

/// Parses an IEX String (fixed-length ASCII byte sequence, left-justified and space-filled on the right)
///
/// # Arguments
///
/// * `length` - The fixed length of the IEX String
///
/// # Example
///
/// ```ignore
/// use nom::error::Error;
/// use crate::utils::iex_string;
///
/// let (_, result) = iex_string(8).unwrap().parse(b"HELLO   ").unwrap();
/// assert_eq!(result, "HELLO");
///
/// let (_, result) = iex_string(8).unwrap().parse(b"        ").unwrap();
/// assert_eq!(result, "");
/// ```
#[inline]
pub fn iex_string<'a, E: ParseError<&'a [u8]>>(
    length: usize,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], &'a str, E> {
    map(take(length), |bytes: &'a [u8]| {
        std::str::from_utf8(bytes)
            .map(|s| s.trim_end())
            .unwrap_or("")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::Error;

    #[test]
    fn test_iex_string_valid() {
        let mut parser = iex_string::<Error<&[u8]>>(8);

        let input = b"HELLO   ";
        let (remaining, result) = parser(input).unwrap();
        assert_eq!(result, "HELLO");
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_iex_string_empty() {
        let mut parser = iex_string::<Error<&[u8]>>(8);

        let input = b"        ";
        let (remaining, result) = parser(input).unwrap();
        assert_eq!(result, "");
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_iex_string_longer_input() {
        let mut parser = iex_string::<Error<&[u8]>>(8);

        let input = b"HELLO   WORLD";
        let (remaining, result) = parser(input).unwrap();
        assert_eq!(result, "HELLO");
        assert_eq!(remaining, b"WORLD");
    }

    #[test]
    fn test_iex_string_invalid_ascii() {
        let mut parser = iex_string::<Error<&[u8]>>(8);

        let input = b"HELLO\xFF\xFF";
        let result = parser(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_iex_string_longer_than_length() {
        let mut parser = iex_string::<Error<&[u8]>>(8);

        let input = b"LONGSTRING";
        let (remaining, result) = parser(input).unwrap();
        assert_eq!(result, "LONGSTRI");
        assert_eq!(remaining, b"NG");
    }
}
