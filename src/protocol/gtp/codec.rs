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

    let sequence = if packet[0] & 0x02 != 0 {
        if packet.len() < 12 {
            return Err(GtpDecodeError::MissingSequence);
        }
        u16::from_be_bytes([packet[8], packet[9]]) as u32
    } else {
        return Err(GtpDecodeError::MissingSequence);
    };

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
    if packet[0] & 0x08 != 0 {
        if packet.len() < 12 {
            return Err(GtpDecodeError::MissingSequence);
        }
        let sequence = ((packet[8] as u32) << 16) | ((packet[9] as u32) << 8) | packet[10] as u32;
        return Ok(GtpEchoReply {
            sequence,
            bytes: packet.len(),
        });
    }

    let sequence = ((packet[4] as u32) << 16) | ((packet[5] as u32) << 8) | packet[6] as u32;
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
}
