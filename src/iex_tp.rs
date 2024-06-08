use chrono::{DateTime, Utc};
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    combinator::map,
    multi::count,
    number::complete::{le_i64, le_u16, le_u32},
    IResult, Parser as _,
};

use crate::utils;

fn iex_tp_1_message(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, length) = le_u16.parse(input)?;
    take(length).parse(input)
}

#[derive(Clone, Debug)]
pub struct IexTp1Segment<'a> {
    pub message_protocol_id: u16,
    pub channel_id: u32,
    pub session_id: u32,
    pub send_time: DateTime<Utc>,
    pub messages: Vec<&'a [u8]>,
    pub first_message_sequence_no: i64,
}

fn iex_tp_1_segment(input: &[u8]) -> IResult<&[u8], IexTp1Segment> {
    // Parse the version (0x01) and the reserved byte
    let (input, _) = tag([1u8, 0u8]).parse(input)?;
    let (input, message_protocol_id) = le_u16.parse(input)?;
    let (input, channel_id) = le_u32.parse(input)?;
    let (input, session_id) = le_u32.parse(input)?;
    let (input, payload_length) = le_u16.parse(input)?;
    let (input, message_count) = le_u16.parse(input)?;
    let (input, _stream_offset) = le_i64.parse(input)?; // We don't verify correctness using stream bytes
    let (input, first_message_sequence_no) = le_i64.parse(input)?;
    let (input, send_time) = utils::timestamp.parse(input)?;

    let (input, payload) = take(payload_length)(input)?;
    let (litter, messages) = count(iex_tp_1_message, message_count.into()).parse(payload)?;
    assert!(litter.is_empty());

    Ok((
        input,
        IexTp1Segment {
            message_protocol_id,
            channel_id,
            session_id,
            send_time,
            messages,
            first_message_sequence_no,
        },
    ))
}

#[derive(Debug)]
pub enum IexTpSegment<'a> {
    V1(IexTp1Segment<'a>),
}

// Parse an outbound IEX-TP segment
pub fn iex_tp_segment(input: &[u8]) -> IResult<&[u8], IexTpSegment> {
    alt((map(iex_tp_1_segment, IexTpSegment::V1),)).parse(input)
    // todo!();
    // Ok((input, IexTpSegment { ??? }))
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use crate::message_protocol_ids;

    use super::*;

    #[test]
    fn provided_example() {
        let input: [u8; 0x70] = [
            0x01, 0x00, 0x04, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x87, 0x42, 0x48, 0x00,
            0x02, 0x00, 0x8C, 0xA6, 0x21, 0x00, 0x00, 0x00, 0x00, 0x00, 0xCA, 0xC3, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xEC, 0x45, 0xC2, 0x20, 0x96, 0x86, 0x6D, 0x14, 0x26, 0x00,
            0x54, 0x00, 0xAC, 0x63, 0xC0, 0x20, 0x96, 0x86, 0x6D, 0x14, 0x5A, 0x49, 0x45, 0x58,
            0x54, 0x20, 0x20, 0x20, 0x64, 0x00, 0x00, 0x00, 0x24, 0x1D, 0x0F, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x96, 0x8F, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1E, 0x00, 0x38, 0x01,
            0xAC, 0x63, 0xC0, 0x20, 0x96, 0x86, 0x6D, 0x14, 0x5A, 0x49, 0x45, 0x58, 0x54, 0x20,
            0x20, 0x20, 0xE4, 0x25, 0x00, 0x00, 0x24, 0x1D, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let result = iex_tp_segment(&input).unwrap();

        assert_matches!(
            result,
            (
                [],
                IexTpSegment::V1(IexTp1Segment {
                    message_protocol_id: message_protocol_ids::DEEP_1_0,
                    channel_id: 1,
                    session_id: 0x42870000,
                    send_time: _,
                    messages: _,
                    first_message_sequence_no: 50122,
                })
            )
        );

        let IexTpSegment::V1(inner_result) = result.1;
        assert_eq!(
            inner_result.send_time,
            DateTime::from_timestamp_nanos(1471980632572839404) // 2016-08-23 15:30:32.572839404
        );

        assert_eq!(inner_result.messages.len(), 2);
        assert_eq!(
            inner_result.messages[0],
            [
                0x54, 0x00, 0xAC, 0x63, 0xC0, 0x20, 0x96, 0x86, 0x6D, 0x14, 0x5A, 0x49, 0x45, 0x58,
                0x54, 0x20, 0x20, 0x20, 0x64, 0x00, 0x00, 0x00, 0x24, 0x1D, 0x0F, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x96, 0x8F, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]
        );
        assert_eq!(
            inner_result.messages[1],
            [
                0x38, 0x01, 0xAC, 0x63, 0xC0, 0x20, 0x96, 0x86, 0x6D, 0x14, 0x5A, 0x49, 0x45, 0x58,
                0x54, 0x20, 0x20, 0x20, 0xE4, 0x25, 0x00, 0x00, 0x24, 0x1D, 0x0F, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ]
        );
    }
}
