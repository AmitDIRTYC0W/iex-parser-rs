use chrono::{DateTime, Utc};
use nom::{
    bits,
    branch::alt,
    bytes::complete::{tag, take},
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

// Handle known yet unimplemented message types
macro_rules! dummy_message_parser {
    ($tag:expr, $len:expr, $msg_type:ident) => {
        fn $msg_type(input: &[u8]) -> IResult<&[u8], ()> {
            let (input, _) = tag($tag).parse(input)?;
            let (input, _) = take($len).parse(input)?;
            Ok((input, ()))
        }
    };
}

dummy_message_parser!([0x53], 9usize, system_event);
dummy_message_parser!([0x44], 30usize, security_directory);
dummy_message_parser!([0x48], 21usize, trading_status);
dummy_message_parser!([0x49], 17usize, retail_liquidity_indicator);
dummy_message_parser!([0x4f], 17usize, operational_halt_status);
dummy_message_parser!([0x50], 18usize, short_sale_price_test_status);
dummy_message_parser!([0x54], 37usize, trade_report);
dummy_message_parser!([0x58], 25usize, official_price);
dummy_message_parser!([0x42], 37usize, trade_break);
dummy_message_parser!([0x41], 79usize, auction_information);

fn quote_update<S>(input: &[u8]) -> IResult<&[u8], QuoteUpdate<S>>
where
    S: for<'a> From<&'a str>,
{
    let (input, _) = tag([0x51]).parse(input)?;
    // let (input, _) = bits::<_, _, Error<(&[u8], usize)>, _, _>(nom::bits::complete::take(4usize))
    //     .parse(input)?;
    let (input, (availability, market_session, _)): (&[u8], (bool, bool, u8)) =
        bits::<_, _, Error<(&[u8], usize)>, _, _>(tuple((
            nom::bits::complete::bool,
            nom::bits::complete::bool,
            nom::bits::complete::tag(0u8, 6usize),
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
    SystemEvent,
    SecurityDirectory,
    TradingStatus,
    RetailLiquidityIndicator,
    OperationalHaltStatus,
    ShortSalePriceTestStatus,
    QuoteUpdate(QuoteUpdate<S>),
    TradeReport,
    OfficialPrice,
    TradeBreak,
    AuctionInformation,
}

pub fn tops_1_6_message<S>(input: &[u8]) -> IResult<&[u8], Tops1_6Message<S>>
where
    S: for<'a> From<&'a str>,
{
    alt((
        map(system_event, |_| Tops1_6Message::SystemEvent),
        map(security_directory, |_| Tops1_6Message::SecurityDirectory),
        map(trading_status, |_| Tops1_6Message::TradingStatus),
        map(retail_liquidity_indicator, |_| {
            Tops1_6Message::RetailLiquidityIndicator
        }),
        map(operational_halt_status, |_| {
            Tops1_6Message::OperationalHaltStatus
        }),
        map(short_sale_price_test_status, |_| {
            Tops1_6Message::ShortSalePriceTestStatus
        }),
        map(quote_update::<S>, Tops1_6Message::QuoteUpdate),
        map(trade_report, |_| Tops1_6Message::TradeReport),
        map(official_price, |_| Tops1_6Message::OfficialPrice),
        map(trade_break, |_| Tops1_6Message::TradeBreak),
        map(auction_information, |_| Tops1_6Message::AuctionInformation),
    ))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use float_eq::assert_float_eq;

    use super::*;

    #[test]
    fn quote_update_example() {
        let input: [u8; 42] = [
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

        if let Tops1_6Message::QuoteUpdate(inner_result) = result.1 {
            assert_eq!(inner_result.symbol, "ZIEXT");

            assert_eq!(
                inner_result.timestamp,
                DateTime::from_timestamp_nanos(1471980632572715948)
            );

            assert_float_eq!(inner_result.bid_price, 99.05, ulps <= 5);
            assert_float_eq!(inner_result.ask_price, 99.07, ulps <= 5);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn quote_with_set_flags() {
        let input: [u8; 42] = [
            81, 192, 130, 69, 230, 110, 149, 21, 218, 23, 65, 72, 73, 32, 32, 32, 32, 32, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let _ = tops_1_6_message::<String>(&input).unwrap();
    }

    #[test]
    fn system_event_message() {
        let input: [u8; 10] = [83, 79, 201, 234, 221, 110, 149, 21, 218, 23];

        let _ = tops_1_6_message::<String>(&input).unwrap();
    }
}
