use chrono::{DateTime, Utc};
use nom::{
    bits,
    branch::alt,
    bytes::complete::tag,
    combinator::map,
    error::Error,
    number::complete::{le_i64, le_u32},
    sequence::{tuple, Tuple as _},
    IResult, Parser as _,
};

use crate::utils;

#[derive(Debug)]
pub enum MarketSession {
    Regular,
    OutOfHours,
}

#[derive(Debug)]
pub struct QuoteUpdate<S>
where
    S: for<'a> From<&'a str>,
{
    pub available: bool,
    pub market_session: MarketSession,
    pub timestamp: DateTime<Utc>,
    pub symbol: S,
    pub bid_size: u32,
    pub bid_price: f64,
    pub ask_size: u32,
    pub ask_price: f64,
}

// TODO document properly
// Price: 8 bytes, signed integer containing a fixed-point number with 4 digits to the right of an implied decimal
// point
fn price(input: &[u8]) -> IResult<&[u8], f64> {
    let (input, int_price) = le_i64.parse(input)?;
    Ok((input, (int_price as f64) * 1e-4))
}

fn quote_update<S>(input: &[u8]) -> IResult<&[u8], QuoteUpdate<S>>
where
    S: for<'a> From<&'a str>,
{
    let (input, _) = tag([0x51]).parse(input)?;
    // let (input, _) = bits::<_, _, Error<(&[u8], usize)>, _, _>(nom::bits::complete::take(4usize))
    //     .parse(input)?;
    let (input, (_, availability, market_session, _)): (&[u8], (u8, bool, bool, u8)) =
        bits::<_, _, Error<(&[u8], usize)>, _, _>(tuple((
            nom::bits::complete::tag(0u8, 1usize),
            nom::bits::complete::bool,
            nom::bits::complete::bool,
            nom::bits::complete::tag(0u8, 5usize),
        )))
        .parse(input)?;
    let (input, timestamp) = utils::timestamp.parse(input)?;
    let (input, symbol) = utils::iex_string(8).parse(input)?;
    let (input, (bid_size, bid_price)) = (le_u32, price).parse(input)?;
    let (input, (ask_price, ask_size)) = (price, le_u32).parse(input)?;

    Ok((
        input,
        QuoteUpdate {
            available: !availability,
            market_session: if market_session {
                MarketSession::OutOfHours
            } else {
                MarketSession::Regular
            },
            timestamp,
            symbol: symbol.into(),
            bid_size,
            bid_price,
            ask_size,
            ask_price,
        },
    ))
}

#[derive(Debug)]
pub enum Tops1_6Message<S>
where
    S: for<'a> From<&'a str>,
{
    QuoteUpdate(QuoteUpdate<S>),
}

pub fn tops_1_6_message<S>(input: &[u8]) -> IResult<&[u8], Tops1_6Message<S>>
where
    S: for<'a> From<&'a str>,
{
    alt((map(quote_update, Tops1_6Message::QuoteUpdate),)).parse(input)
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use float_eq::assert_float_eq;

    use super::*;

    #[test]
    fn quote_update_example() {
        let input: [u8; 0x2A] = [
            0x51, 0x00, 0xAC, 0x63, 0xC0, 0x20, 0x96, 0x86, 0x6D, 0x14, 0x5A, 0x49, 0x45, 0x58,
            0x54, 0x20, 0x20, 0x20, 0xE4, 0x25, 0x00, 0x00, 0x24, 0x1D, 0x0F, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xEC, 0x1D, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE8, 0x03, 0x00, 0x00,
        ];
        let result = tops_1_6_message::<String>(&input).unwrap();

        assert_matches!(
            result,
            (
                [],
                Tops1_6Message::QuoteUpdate(QuoteUpdate {
                    available: true,
                    market_session: MarketSession::Regular,
                    timestamp: _,
                    symbol: _,
                    bid_size: 9700,
                    bid_price: _,
                    ask_size: 1000,
                    ask_price: _,
                })
            )
        );

        let Tops1_6Message::QuoteUpdate(inner_result) = result.1;

        assert_eq!(inner_result.symbol, "ZIEXT");

        assert_eq!(
            inner_result.timestamp,
            DateTime::from_timestamp_nanos(1471980632572715948)
        );

        assert_float_eq!(inner_result.bid_price, 99.05, ulps <= 5);
        assert_float_eq!(inner_result.ask_price, 99.07, ulps <= 5);
    }
}
