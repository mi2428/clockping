use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GtpCodec {
    V1,
    V2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GtpEchoReply {
    pub sequence: u32,
    pub bytes: usize,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum GtpDecodeError {
    #[error("packet too short")]
    TooShort,
    #[error("unsupported GTP version")]
    UnsupportedVersion,
    #[error("not an echo response")]
    NotEchoResponse,
    #[error("missing sequence number")]
    MissingSequence,
    #[error("invalid GTP header or declared length")]
    InvalidHeader,
}

impl GtpCodec {
    pub fn sequence_from_u64(self, seq: u64) -> u32 {
        match self {
            Self::V1 => (seq & 0xffff) as u32,
            Self::V2 => (seq & 0x00ff_ffff) as u32,
        }
    }

    pub fn encode_echo_request(self, sequence: u32) -> Vec<u8> {
        match self {
            Self::V1 => encode_v1_echo_request(sequence as u16),
            Self::V2 => encode_v2_echo_request(sequence),
        }
    }

    pub fn decode_echo_reply(self, packet: &[u8]) -> Result<GtpEchoReply, GtpDecodeError> {
        match self {
            Self::V1 => decode_v1_echo_reply(packet),
            Self::V2 => decode_v2_echo_reply(packet),
        }
    }
}

fn encode_v1_echo_request(sequence: u16) -> Vec<u8> {
    let mut packet = Vec::with_capacity(12);
    packet.push(0x32); // Version 1, protocol type GTP, sequence number present.
    packet.push(1); // Echo Request.
    packet.extend_from_slice(&4_u16.to_be_bytes());
    packet.extend_from_slice(&0_u32.to_be_bytes()); // TEID is zero for Echo.
    packet.extend_from_slice(&sequence.to_be_bytes());
    packet.push(0); // N-PDU number.
    packet.push(0); // Next extension header type.
    packet
}

fn decode_v1_echo_reply(packet: &[u8]) -> Result<GtpEchoReply, GtpDecodeError> {
    if packet.len() < 8 {
        return Err(GtpDecodeError::TooShort);
    }
    if packet[0] >> 5 != 1 {
        return Err(GtpDecodeError::UnsupportedVersion);
    }
    if packet[1] != 2 {
        return Err(GtpDecodeError::NotEchoResponse);
    }
    let declared = u16::from_be_bytes([packet[2], packet[3]]) as usize;
    if packet.len() != 8 + declared {
        return Err(GtpDecodeError::InvalidHeader);
    }

    let sequence = if packet[0] & 0x02 != 0 {
        if declared < 4 {
            return Err(GtpDecodeError::MissingSequence);
        }
        u16::from_be_bytes([packet[8], packet[9]]) as u32
    } else {
        return Err(GtpDecodeError::MissingSequence);
    };
    // TS 29.060 clause 6: unused optional fields and spare bits are ignored.
    let mut next = if packet[0] & 0x04 != 0 { packet[11] } else { 0 };
    let mut offset = 12;
    while next != 0 {
        let size = packet.get(offset).copied().unwrap_or(0) as usize * 4;
        if size < 4 || offset + size > packet.len() {
            return Err(GtpDecodeError::InvalidHeader);
        }
        next = packet[offset + size - 1];
        offset += size;
    }

    Ok(GtpEchoReply {
        sequence,
        bytes: packet.len(),
    })
}

fn encode_v2_echo_request(sequence: u32) -> Vec<u8> {
    let seq = sequence & 0x00ff_ffff;
    let mut packet = Vec::with_capacity(8);
    packet.push(0x40); // Version 2, no TEID.
    packet.push(1); // Echo Request.
    packet.extend_from_slice(&4_u16.to_be_bytes());
    packet.push(((seq >> 16) & 0xff) as u8);
    packet.push(((seq >> 8) & 0xff) as u8);
    packet.push((seq & 0xff) as u8);
    packet.push(0); // Spare.
    packet
}

fn decode_v2_echo_reply(packet: &[u8]) -> Result<GtpEchoReply, GtpDecodeError> {
    if packet.len() < 8 {
        return Err(GtpDecodeError::TooShort);
    }
    if packet[0] >> 5 != 2 {
        return Err(GtpDecodeError::UnsupportedVersion);
    }
    if packet[1] != 2 {
        return Err(GtpDecodeError::NotEchoResponse);
    }
    let sequence_offset = if packet[0] & 0x08 != 0 { 8 } else { 4 };
    if (u16::from_be_bytes([packet[2], packet[3]]) as usize) < sequence_offset {
        return Err(GtpDecodeError::MissingSequence);
    }
    let mut offset = 0;
    loop {
        let frame = &packet[offset..];
        if frame.len() < 8 || frame[0] >> 5 != 2 {
            return Err(GtpDecodeError::InvalidHeader);
        }
        let size = 4 + u16::from_be_bytes([frame[2], frame[3]]) as usize;
        if size < if frame[0] & 0x08 != 0 { 12 } else { 8 } || size > frame.len() {
            return Err(GtpDecodeError::InvalidHeader);
        }
        offset += size;
        if frame[0] & 0x10 == 0 {
            if offset != packet.len() {
                return Err(GtpDecodeError::InvalidHeader);
            }
            break;
        }
        if offset == packet.len() {
            return Err(GtpDecodeError::InvalidHeader);
        }
    }

    let sequence = ((packet[sequence_offset] as u32) << 16)
        | ((packet[sequence_offset + 1] as u32) << 8)
        | packet[sequence_offset + 2] as u32;
    Ok(GtpEchoReply {
        sequence,
        bytes: packet.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_echo_request_shape() {
        let packet = GtpCodec::V1.encode_echo_request(0x1234);
        assert_eq!(
            packet,
            vec![0x32, 0x01, 0x00, 0x04, 0, 0, 0, 0, 0x12, 0x34, 0, 0]
        );
    }

    #[test]
    fn v1_echo_response_decodes_sequence() {
        let packet = vec![0x32, 0x02, 0x00, 0x04, 0, 0, 0, 0, 0xab, 0xcd, 0, 0];
        let reply = GtpCodec::V1.decode_echo_reply(&packet).unwrap();
        assert_eq!(reply.sequence, 0xabcd);
        assert_eq!(reply.bytes, packet.len());
    }

    #[test]
    fn v2_echo_request_shape() {
        let packet = GtpCodec::V2.encode_echo_request(0x123456);
        assert_eq!(packet, vec![0x40, 0x01, 0x00, 0x04, 0x12, 0x34, 0x56, 0]);
    }

    #[test]
    fn v2_echo_response_decodes_sequence_without_teid() {
        let packet = vec![0x40, 0x02, 0x00, 0x04, 0x12, 0x34, 0x56, 0];
        let reply = GtpCodec::V2.decode_echo_reply(&packet).unwrap();
        assert_eq!(reply.sequence, 0x123456);
    }

    #[test]
    fn rejects_malformed_echo_headers_and_lengths() {
        let v1 = [0x32, 2, 0, 4, 0, 0, 0, 0, 0x12, 0x34, 0, 0];
        let v2 = [0x40, 2, 0, 4, 0x12, 0x34, 0x56, 0];
        for (codec, valid) in [(GtpCodec::V1, v1.as_slice()), (GtpCodec::V2, v2.as_slice())] {
            assert!(codec.decode_echo_reply(valid).is_ok());
            for length in [0_u16, 3, 0xffff] {
                let mut packet = valid.to_vec();
                packet[2..4].copy_from_slice(&length.to_be_bytes());
                assert!(
                    codec.decode_echo_reply(&packet).is_err(),
                    "{codec:?}: {length}"
                );
            }
            assert!(codec.decode_echo_reply(&valid[..valid.len() - 1]).is_err());
        }
        let mut invalid = v1;
        invalid[0] |= 0x04; // Extension bit with a truncated extension header.
        invalid[11] = 0x85;
        assert!(GtpCodec::V1.decode_echo_reply(&invalid).is_err());
        let mut invalid = v2;
        invalid[0] |= 0x08; // TEID layout without the required sequence/header bytes.
        assert!(GtpCodec::V2.decode_echo_reply(&invalid).is_err());
        invalid = v2;
        invalid[0] |= 0x10; // Piggyback flag without a subsequent message.
        assert!(GtpCodec::V2.decode_echo_reply(&invalid).is_err());

        let mut extended_v1 = v1.to_vec();
        extended_v1[0] |= 0x04;
        extended_v1[11] = 0x85;
        extended_v1[3] = 8;
        extended_v1.extend_from_slice(&[1, 0, 0, 0]);
        assert!(GtpCodec::V1.decode_echo_reply(&extended_v1).is_ok());
        let mut piggyback_v2 = v2.to_vec();
        piggyback_v2[0] |= 0x10;
        piggyback_v2.extend_from_slice(&v2);
        assert!(GtpCodec::V2.decode_echo_reply(&piggyback_v2).is_ok());
    }

    #[test]
    fn ignores_spare_and_unused_fields_and_preserves_header_layouts() {
        // TS 29.060 clause 6 and TS 29.274 clauses 5.1/5.3 require receivers
        // to ignore spare bits; E=0 also means the next-extension field is ignored.
        let v1 = [0x3a, 2, 0, 6, 0, 0, 0, 0, 0x12, 0x34, 0xff, 0xff, 14, 0];
        assert_eq!(
            GtpCodec::V1.decode_echo_reply(&v1).unwrap().sequence,
            0x1234
        );
        let v2 = [0x47, 2, 0, 9, 0x12, 0x34, 0x56, 0xff, 3, 0, 1, 0, 0];
        assert_eq!(
            GtpCodec::V2.decode_echo_reply(&v2).unwrap().sequence,
            0x123456
        );

        // Retain the previously supported T=1 layout while validating its length.
        // Echo senders use T=0 (TS 29.274 5.3); this codec only validates framing.
        let with_teid = [0x48, 2, 0, 8, 0, 0, 0, 1, 0x12, 0x34, 0x56, 0xff];
        assert_eq!(
            GtpCodec::V2.decode_echo_reply(&with_teid).unwrap().sequence,
            0x123456
        );
        let mut piggyback = v2.to_vec();
        piggyback[0] |= 0x10;
        let mut second = with_teid;
        second[0] |= 0x07;
        second[1] = 95; // Create Bearer Request: T=1 is a valid non-Echo layout.
        piggyback.extend_from_slice(&second);
        assert_eq!(
            GtpCodec::V2.decode_echo_reply(&piggyback).unwrap().sequence,
            0x123456
        );
    }
}
