/// Canonical 24-byte IPC Protocol Header
/// 
/// Wire format (Little Endian):
/// - Bytes 0-3:   Magic number (0x4C415043 = "LAPC")
/// - Byte 4:      Protocol version (1)
/// - Byte 5:      Flags (reserved)
/// - Bytes 6-7:   Message type (u16)
/// - Bytes 8-11:  Payload length (u32)
/// - Bytes 12-19: Message ID (u64)
/// - Bytes 20-23: CRC32 checksum (entire message including header+payload)

use anyhow::{bail, Result};

pub const HEADER_SIZE: usize = 24;
pub const MAGIC_HEADER: u32 = 0x4C415043;  // "LAPC" in LE
pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;  // 10MB

/// Message types
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Heartbeat = 1,
    Data = 2,
    Control = 3,
    Response = 4,
}

impl MessageType {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            1 => Some(MessageType::Heartbeat),
            2 => Some(MessageType::Data),
            3 => Some(MessageType::Control),
            4 => Some(MessageType::Response),
            _ => None,
        }
    }
}

/// Canonical 24-byte header
#[repr(C)]
#[derive(Debug, Clone)]
pub struct CanonicalHeader {
    pub magic: u32,
    pub version: u8,
    pub flags: u8,
    pub msg_type: u16,
    pub payload_len: u32,
    pub message_id: u64,
    pub crc32: u32,
}

impl CanonicalHeader {
    /// Create a new header
    pub fn new(msg_type: MessageType, payload_len: u32, message_id: u64) -> Self {
        Self {
            magic: MAGIC_HEADER,
            version: PROTOCOL_VERSION,
            flags: 0,
            msg_type: msg_type as u16,
            payload_len,
            message_id,
            crc32: 0,  // Computed after serialization
        }
    }

    /// Encode header to bytes (LE format)
    pub fn encode(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..4].copy_from_slice(&self.magic.to_le_bytes());
        bytes[4] = self.version;
        bytes[5] = self.flags;
        bytes[6..8].copy_from_slice(&self.msg_type.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.payload_len.to_le_bytes());
        bytes[12..20].copy_from_slice(&self.message_id.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.crc32.to_le_bytes());
        bytes
    }

    /// Decode header from bytes (LE format)
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            bail!("Insufficient bytes for header: {} < {}", bytes.len(), HEADER_SIZE);
        }

        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != MAGIC_HEADER {
            bail!("Invalid magic number: 0x{:08X}, expected 0x{:08X}", magic, MAGIC_HEADER);
        }

        let version = bytes[4];
        if version != PROTOCOL_VERSION {
            bail!("Unsupported protocol version: {}, expected {}", version, PROTOCOL_VERSION);
        }

        let payload_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        if payload_len as usize > MAX_MESSAGE_SIZE {
            bail!("Payload too large: {} > {}", payload_len, MAX_MESSAGE_SIZE);
        }

        Ok(Self {
            magic,
            version,
            flags: bytes[5],
            msg_type: u16::from_le_bytes([bytes[6], bytes[7]]),
            payload_len,
            message_id: u64::from_le_bytes([
                bytes[12], bytes[13], bytes[14], bytes[15],
                bytes[16], bytes[17], bytes[18], bytes[19],
            ]),
            crc32: u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
        })
    }

    /// Compute CRC32 over entire message (header + payload)
    pub fn compute_crc32(&self, payload: &[u8]) -> u32 {
        let mut full_msg = Vec::with_capacity(HEADER_SIZE + payload.len());
        
        // Encode header with CRC32 = 0
        let mut temp_header = self.clone();
        temp_header.crc32 = 0;
        full_msg.extend_from_slice(&temp_header.encode());
        full_msg.extend_from_slice(payload);
        
        crc32fast::hash(&full_msg)
    }

    /// Validate CRC32 against payload
    pub fn validate_crc32(&self, payload: &[u8]) -> Result<()> {
        let computed = self.compute_crc32(payload);
        if computed != self.crc32 {
            bail!("CRC32 mismatch: computed 0x{:08X}, expected 0x{:08X}", computed, self.crc32);
        }
        Ok(())
    }

    /// Create and encode a complete message (header + payload) with CRC32
    pub fn encode_message(msg_type: MessageType, payload: &[u8], message_id: u64) -> Result<Vec<u8>> {
        if payload.len() > MAX_MESSAGE_SIZE {
            bail!("Payload too large: {} > {}", payload.len(), MAX_MESSAGE_SIZE);
        }

        let mut header = Self::new(msg_type, payload.len() as u32, message_id);
        header.crc32 = header.compute_crc32(payload);

        let mut encoded = Vec::with_capacity(HEADER_SIZE + payload.len());
        encoded.extend_from_slice(&header.encode());
        encoded.extend_from_slice(payload);
        Ok(encoded)
    }

    /// Decode and validate a complete message (header + payload)
    pub fn decode_message(data: &[u8]) -> Result<(Self, Vec<u8>)> {
        if data.len() < HEADER_SIZE {
            bail!("Message too short: {} < {}", data.len(), HEADER_SIZE);
        }

        let header = Self::decode(&data[..HEADER_SIZE])?;
        
        let expected_total_len = HEADER_SIZE + header.payload_len as usize;
        if data.len() < expected_total_len {
            bail!("Incomplete message: {} < {}", data.len(), expected_total_len);
        }

        let payload = data[HEADER_SIZE..expected_total_len].to_vec();
        header.validate_crc32(&payload)?;

        Ok((header, payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_encode_decode() {
        let header = CanonicalHeader::new(MessageType::Data, 100, 12345);
        let encoded = header.encode();
        
        let decoded = CanonicalHeader::decode(&encoded).unwrap();
        assert_eq!(decoded.magic, MAGIC_HEADER);
        assert_eq!(decoded.version, PROTOCOL_VERSION);
        assert_eq!(decoded.msg_type, MessageType::Data as u16);
        assert_eq!(decoded.payload_len, 100);
        assert_eq!(decoded.message_id, 12345);
    }

    #[test]
    fn test_crc32_validation() {
        let payload = b"Hello, World!";
        let encoded = CanonicalHeader::encode_message(MessageType::Data, payload, 1).unwrap();
        
        let (header, decoded_payload) = CanonicalHeader::decode_message(&encoded).unwrap();
        assert_eq!(decoded_payload, payload);
        assert_eq!(header.message_id, 1);
    }

    #[test]
    fn test_bad_magic() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
        
        let result = CanonicalHeader::decode(&bytes);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid magic"));
    }

    #[test]
    fn test_bad_version() {
        let mut header = CanonicalHeader::new(MessageType::Data, 0, 1);
        header.version = 99;
        let bytes = header.encode();
        
        let result = CanonicalHeader::decode(&bytes);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported protocol version"));
    }

    #[test]
    fn test_crc32_mismatch() {
        let payload = b"Hello";
        let mut encoded = CanonicalHeader::encode_message(MessageType::Data, payload, 1).unwrap();
        
        // Corrupt the CRC32
        encoded[20] ^= 0xFF;
        
        let result = CanonicalHeader::decode_message(&encoded);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("CRC32 mismatch"));
    }

    #[test]
    fn test_oversized_payload() {
        let payload = vec![0u8; MAX_MESSAGE_SIZE + 1];
        let result = CanonicalHeader::encode_message(MessageType::Data, &payload, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Payload too large"));
    }
}
