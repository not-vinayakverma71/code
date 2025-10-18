/// Zero-copy codec integration with tokio_util::codec
/// Implements FramedRead/Write for BinaryCodec with no intermediate copies

use bytes::{Bytes, BytesMut, BufMut};
use tokio_util::codec::{Decoder, Encoder};
use anyhow::{Result, bail};
use std::io;
use rkyv::Deserialize;

use super::binary_codec::{
    BinaryCodec, Message, MessageType,
    MAGIC_HEADER, PROTOCOL_VERSION, FLAG_COMPRESSED, MAX_MESSAGE_SIZE,
    HEADER_SIZE
};

/// Zero-copy codec for IPC messages
pub struct ZeroCopyCodec {
    codec: BinaryCodec,
    max_frame_size: usize,
    enforce_compression: bool,
}

impl ZeroCopyCodec {
    pub fn new() -> Self {
        Self {
            codec: BinaryCodec::new(),
            max_frame_size: MAX_MESSAGE_SIZE,
            enforce_compression: false,
        }
    }
    
    pub fn with_compression(mut self, enforce: bool) -> Self {
        self.enforce_compression = enforce;
        self
    }
    
    pub fn with_max_frame_size(mut self, size: usize) -> Self {
        self.max_frame_size = size;
        self
    }
}

impl Decoder for ZeroCopyCodec {
    type Item = Message;
    type Error = io::Error;
    
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least header size (24 bytes)
        if src.len() < HEADER_SIZE {
            return Ok(None);
        }
        
        // Peek at header without consuming
        let header = &src[..HEADER_SIZE];
        
        // Validate magic (Little-Endian)
        let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        if magic != MAGIC_HEADER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid magic: {:#x}, expected {:#x}", magic, MAGIC_HEADER)
            ));
        }
        
        // Check version
        if header[4] != PROTOCOL_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported protocol version: {}", header[4])
            ));
        }
        
        // Get flags
        let flags = header[5];
        let compressed = (flags & FLAG_COMPRESSED) != 0;
        
        // Get message type (Little-Endian)
        let msg_type_raw = u16::from_le_bytes([header[6], header[7]]);
        
        // Get payload length (Little-Endian)
        let payload_len = u32::from_le_bytes([
            header[8], header[9], header[10], header[11]
        ]) as usize;
        
        // Check max message size
        if payload_len > self.max_frame_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Message too large: {} > {}", payload_len, self.max_frame_size)
            ));
        }
        
        // Get message ID (Little-Endian)
        let msg_id = u64::from_le_bytes([
            header[12], header[13], header[14], header[15],
            header[16], header[17], header[18], header[19]
        ]);
        
        // Get CRC32 (Little-Endian)
        let crc32_received = u32::from_le_bytes([
            header[20], header[21], header[22], header[23]
        ]);
        
        // Check if we have full message
        let total_size = HEADER_SIZE + payload_len;
        if src.len() < total_size {
            // Need more data
            return Ok(None);
        }
        
        // Extract full message (zero-copy via split_to)
        let message_data = src.split_to(total_size);
        
        // Verify CRC32
        let mut check_data = message_data.to_vec();
        check_data[20..24].fill(0); // Zero out CRC field for calculation
        let calculated_crc = crc32fast::hash(&check_data);
        if calculated_crc != crc32_received {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("CRC32 mismatch: expected {:#x}, got {:#x}", crc32_received, calculated_crc)
            ));
        }
        
        // Get payload slice
        let payload_bytes = &message_data[HEADER_SIZE..];
        
        // Decode payload based on compression flag
        let payload_data = if compressed {
            // Decompress using zstd
            match zstd::decode_all(payload_bytes) {
                Ok(decompressed) => Bytes::from(decompressed),
                Err(e) => return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Decompression failed: {}", e)
                )),
            }
        } else {
            // Zero-copy
            Bytes::copy_from_slice(payload_bytes)
        };
        
        // Deserialize full message using rkyv (zero-copy deserialization)
        let archived = match rkyv::check_archived_root::<Message>(&payload_data) {
            Ok(archived) => archived,
            Err(e) => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid archived data: {}", e)
            )),
        };
        
        let message: Message = match archived.deserialize(&mut rkyv::Infallible) {
            Ok(msg) => msg,
            Err(_) => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Failed to deserialize message"
            )),
        };
        
        // Verify message type matches header
        let msg_type = match MessageType::try_from(msg_type_raw) {
            Ok(t) => t,
            Err(_) => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid message type: {:#x}", msg_type_raw)
            )),
        };
        
        if message.msg_type != msg_type {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Message type mismatch: header={:?}, payload={:?}", msg_type, message.msg_type)
            ));
        }
        
        // Verify message ID matches
        if message.id != msg_id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Message ID mismatch: header={}, payload={}", msg_id, message.id)
            ));
        }
        
        Ok(Some(message))
    }
}

impl Encoder<Message> for ZeroCopyCodec {
    type Error = io::Error;
    
    fn encode(&mut self, msg: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize full message with rkyv
        let archived = rkyv::to_bytes::<_, 256>(&msg)
            .map_err(|e| io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Serialization failed: {}", e)
            ))?;
        
        let payload_bytes = archived.as_slice();
        let mut payload_len = payload_bytes.len();
        
        // Check size limit
        if payload_len > self.max_frame_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Message too large: {} bytes", payload_len)
            ));
        }
        
        // Determine if compression needed
        let should_compress = self.enforce_compression && payload_len > 1024;
        let mut flags = 0u8;
        
        // Compress if needed
        let final_payload = if should_compress {
            let compressed = zstd::encode_all(payload_bytes, 3)
                .map_err(|e| io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Compression failed: {}", e)
                ))?;
            payload_len = compressed.len();
            flags |= FLAG_COMPRESSED;
            compressed
        } else {
            payload_bytes.to_vec()
        };
        
        // Reserve space for header + payload
        dst.reserve(HEADER_SIZE + payload_len);
        
        // Write header (24 bytes) - all Little-Endian as per spec
        dst.put_u32_le(MAGIC_HEADER);
        dst.put_u8(PROTOCOL_VERSION);
        dst.put_u8(flags);
        dst.put_u16_le(msg.msg_type as u16);
        dst.put_u32_le(payload_len as u32);
        dst.put_u64_le(msg.id);
        
        // Reserve space for CRC32
        let crc_offset = dst.len();
        dst.put_u32_le(0);
        
        // Write payload
        dst.extend_from_slice(&final_payload);
        
        // Calculate and write CRC32
        let message_bytes = &dst[dst.len() - HEADER_SIZE - payload_len..];
        let crc = crc32fast::hash(message_bytes);
        dst[crc_offset..crc_offset+4].copy_from_slice(&crc.to_le_bytes());
        
        Ok(())
    }
}

/// Zero-copy stream wrapper
pub struct ZeroCopyStream<T> {
    inner: tokio_util::codec::Framed<T, ZeroCopyCodec>,
}

impl<T> ZeroCopyStream<T> 
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(inner: T) -> Self {
        Self {
            inner: tokio_util::codec::Framed::new(inner, ZeroCopyCodec::new()),
        }
    }
    
    pub fn with_codec(inner: T, codec: ZeroCopyCodec) -> Self {
        Self {
            inner: tokio_util::codec::Framed::new(inner, codec),
        }
    }
}

impl<T> ZeroCopyStream<T>
where
    T: tokio::io::AsyncRead + Unpin,
{
    pub async fn read_message(&mut self) -> Result<Option<Message>> {
        use futures::StreamExt;
        
        match self.inner.next().await {
            Some(Ok(msg)) => Ok(Some(msg)),
            Some(Err(e)) => bail!("Read error: {}", e),
            None => Ok(None),
        }
    }
}

impl<T> ZeroCopyStream<T>
where
    T: tokio::io::AsyncWrite + Unpin,
{
    pub async fn write_message(&mut self, msg: Message) -> Result<()> {
        use futures::SinkExt;
        
        self.inner.send(msg).await
            .map_err(|e| anyhow::anyhow!("Write error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    #[tokio::test]
    async fn test_zero_copy_roundtrip() {
        let (client, server) = tokio::io::duplex(64 * 1024);
        
        let mut client_stream = ZeroCopyStream::new(client);
        let mut server_stream = ZeroCopyStream::new(server);
        
        // Create test message
        let msg = Message {
            id: 1,
            msg_type: MessageType::CompletionRequest,
            payload: crate::ipc::binary_codec::MessagePayload::CompletionRequest(crate::ipc::binary_codec::CompletionRequest {
                prompt: "test prompt".to_string(),
                max_tokens: 100,
                temperature: 0.7,
                model: "test-model".to_string(),
                stream: false,
            }),
            timestamp: 12345,
        };
        
        // Send from client
        client_stream.write_message(msg.clone()).await.unwrap();
        
        // Receive on server
        let received = server_stream.read_message().await.unwrap().unwrap();
        
        assert_eq!(received.msg_type, msg.msg_type);
        assert_eq!(received.timestamp, msg.timestamp);
    }
    
    #[tokio::test]
    async fn test_compression_enforcement() {
        let (client, server) = tokio::io::duplex(64 * 1024);
        
        let codec = ZeroCopyCodec::new().with_compression(true);
        let mut client_stream = ZeroCopyStream::with_codec(client, codec);
        let mut server_stream = ZeroCopyStream::new(server);
        
        // Large message should be compressed
        let msg = Message {
            id: 2,
            msg_type: MessageType::StreamChunk,
            payload: crate::ipc::binary_codec::MessagePayload::StreamChunk(crate::ipc::binary_codec::StreamChunk {
                stream_id: 1,
                sequence: 1,
                content: crate::ipc::binary_codec::ChunkContent::Text("x".repeat(2048)), // Large payload
                is_final: false,
            }),
            timestamp: 12345,
        };
        
        client_stream.write_message(msg).await.unwrap();
        let received = server_stream.read_message().await.unwrap().unwrap();
        
        // Check message was received correctly
        assert_eq!(received.msg_type, MessageType::StreamChunk);
    }
    
    #[tokio::test]
    async fn test_max_message_size_enforcement() {
        let mut codec = ZeroCopyCodec::new().with_max_frame_size(100);
        let mut buf = BytesMut::new();
        
        // Message too large
        let msg = Message {
            id: 3,
            msg_type: MessageType::StreamChunk,
            payload: crate::ipc::binary_codec::MessagePayload::StreamChunk(crate::ipc::binary_codec::StreamChunk {
                stream_id: 1,
                sequence: 1,
                content: crate::ipc::binary_codec::ChunkContent::Text("x".repeat(1024)), // Exceeds max size
                is_final: false,
            }),
            timestamp: 12345,
        };
        
        let result = codec.encode(msg, &mut buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too large"));
    }
}
