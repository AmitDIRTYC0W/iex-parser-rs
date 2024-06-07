use chrono::{DateTime, Utc};

use nom::{number::complete::le_i64, IResult, Parser as _};

#[inline]
pub fn timestamp(input: &[u8]) -> IResult<&[u8], DateTime<Utc>> {
    let (input, unix_time) = le_i64.parse(input)?;
    Ok((input, DateTime::from_timestamp_nanos(unix_time)))
}
