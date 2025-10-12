/// Framed Shared Memory Stream with Canonical 24-byte Header
/// 
/// Wraps SharedMemoryStream to provide:
/// - Canonical 24-byte header with CRC32 validation
/// - Message type routing
/// - Message ID tracking
/// - rkyv serialization support

use super::canonical_header::{CanonicalHeader, MessageType, HEADER_SIZE};
use super::shared_memory_complete::SharedMemoryStream;
use anyhow::{bail, Result};
use std::sync::atomic::{AtomicU64, Ordering};

/// Framed stream with canonical protocol
pub struct FramedShmStream {
    inner: SharedMemoryStream,
    next_message_id: AtomicU64,
}

impl FramedShmStream {
    /// Create from existing SharedMemoryStream
    pub fn new(inner: SharedMemoryStream) -> Self {
        Self {
            inner,
            next_message_id: AtomicU64::new(1),
        }
    }

    /// Send a message with canonical framing
    pub async fn send(&mut self, msg_type: MessageType, payload: &[u8]) -> Result<u64> {
        let message_id = self.next_message_id.fetch_add(1, Ordering::Relaxed);
        
        // Encode message with canonical header and CRC32
        let encoded = CanonicalHeader::encode_message(msg_type, payload, message_id)?;
        
        // Write to underlying stream
        self.inner.write_all(&encoded).await?;
        
        Ok(message_id)
    }

    /// Receive a message with canonical framing
    pub async fn recv(&mut self) -> Result<(MessageType, Vec<u8>, u64)> {
        // Read header first
        let mut header_bytes = vec![0u8; HEADER_SIZE];
        self.inner.read_exact(&mut header_bytes).await?;
        
        let header = CanonicalHeader::decode(&header_bytes)?;
        
        // Read payload
        let mut payload = vec![0u8; header.payload_len as usize];
        self.inner.read_exact(&mut payload).await?;
        
        // Validate CRC32
        header.validate_crc32(&payload)?;
        
        // Parse message type
        let msg_type = MessageType::from_u16(header.msg_type)
            .ok_or_else(|| anyhow::anyhow!("Unknown message type: {}", header.msg_type))?;
        
        Ok((msg_type, payload, header.message_id))
    }

    /// Send a serializable message using rkyv
    pub async fn send_serialized<T: rkyv::Serialize<rkyv::ser::serializers::AllocSerializer<1024>>>(
        &mut self,
        msg_type: MessageType,
        data: &T,
    ) -> Result<u64> {
        // Serialize with rkyv
        let bytes = rkyv::to_bytes::<_, 1024>(data)
            .map_err(|e| anyhow::anyhow!("rkyv serialization failed: {}", e))?;
        
        self.send(msg_type, &bytes).await
    }

    /// Receive and deserialize a message using rkyv
    /// Note: Simplified to avoid rkyv API version issues
    pub async fn recv_deserialized<T>(&mut self) -> Result<(MessageType, T, u64)>
    where
        T: rkyv::Archive,
        T::Archived: for<'a> rkyv::CheckBytes<rkyv::validation::validators::DefaultValidator<'a>>,
    {
        let (msg_type, payload, message_id) = self.recv().await?;
        
        // Use zero-copy access to archived data
        let archived = unsafe { rkyv::archived_root::<T>(&payload) };
        
        // For now, users should access archived directly or use bytecheck
        // Full deserialization requires matching rkyv version APIs
        anyhow::bail!("recv_deserialized not fully implemented - use recv() and manual deserialization")
    }

    /// Access the underlying SharedMemoryStream
    pub fn inner(&self) -> &SharedMemoryStream {
        &self.inner
    }
    
    /// Access the underlying SharedMemoryStream mutably
    pub fn inner_mut(&mut self) -> &mut SharedMemoryStream {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rkyv::{Archive, Deserialize, Serialize};

    #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
    #[archive(compare(PartialEq))]
    #[archive_attr(derive(Debug))]
    struct TestMessage {
        id: u64,
        name: String,
        count: u32,
    }

    #[tokio::test]
    async fn test_framed_send_recv() {
        // This test requires a mock SharedMemoryStream
        // In production, use actual IPC connection
    }

    #[test]
    fn test_canonical_header_format() {
        // Verify header size
        assert_eq!(HEADER_SIZE, 24);
        
        // Verify we can encode/decode
        let header = CanonicalHeader::new(MessageType::Data, 100, 12345);
        let encoded = header.encode();
        assert_eq!(encoded.len(), HEADER_SIZE);
        
        let decoded = CanonicalHeader::decode(&encoded).unwrap();
        assert_eq!(decoded.payload_len, 100);
        assert_eq!(decoded.message_id, 12345);
    }
}
