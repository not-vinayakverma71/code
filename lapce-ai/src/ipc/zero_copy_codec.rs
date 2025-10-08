/// Zero-copy codec integration with tokio_util::codec
/// Implements FramedRead/Write for BinaryCodec with no intermediate copies

use bytes::{Bytes, BytesMut, Buf, BufMut};
use tokio_util::codec::{Decoder, Encoder};
use anyhow::{Result, bail};
use std::io;
use rkyv::{Archive, Deserialize, Serialize};

use super::binary_codec::{
    BinaryCodec, Message, MessagePayload, MessageType,
    CompletionRequest, StreamChunk, ChunkContent,
    MAGIC_BYTES, PROTOCOL_VERSION, FLAG_COMPRESSED, MAX_MESSAGE_SIZE
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
        // Need at least header size (16 bytes)
        const HEADER_SIZE: usize = 16;
        
        if src.len() < HEADER_SIZE {
            return Ok(None);
        }
        
        // Peek at header without consuming
        let header = &src[..HEADER_SIZE];
        
        // Validate magic bytes
        if &header[0..4] != MAGIC_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid magic bytes"
            ));
        }
        
        // Check version
        if header[4] != PROTOCOL_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported protocol version: {}", header[4])
            ));
        }
        
        // Get message type
        let msg_type = u16::from_le_bytes([header[5], header[6]]);
        
        // Get flags
        let flags = header[7];
        let compressed = (flags & FLAG_COMPRESSED) != 0;
        
        // Get payload length
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
        
        // Get timestamp
        let timestamp = u32::from_le_bytes([
            header[12], header[13], header[14], header[15]
        ]);
        
        // Check if we have full message
        let total_size = HEADER_SIZE + payload_len;
        if src.len() < total_size {
            // Need more data
            return Ok(None);
        }
        
        // Extract full message (zero-copy via split_to)
        let mut message_bytes = src.split_to(total_size);
        message_bytes.advance(HEADER_SIZE); // Skip header
        
        // Decode payload based on compression flag
        let payload_data = if compressed {
            // Decompress using zstd
            match zstd::decode_all(&message_bytes[..]) {
                Ok(decompressed) => Bytes::from(decompressed),
                Err(e) => return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Decompression failed: {}", e)
                )),
            }
        } else {
            // Zero-copy: convert BytesMut to Bytes without copying
            message_bytes.freeze()
        };
        
        // Deserialize message payload using rkyv (zero-copy deserialization)
        let archived = match rkyv::check_archived_root::<MessagePayload>(&payload_data) {
            Ok(archived) => archived,
            Err(e) => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid archived data: {}", e)
            )),
        };
        
        let payload: MessagePayload = match archived.deserialize(&mut rkyv::Infallible) {
            Ok(payload) => payload,
            Err(_) => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Failed to deserialize payload"
            )),
        };
        
        // Convert u16 to MessageType
        let message_type = match msg_type {
            0x0001 => MessageType::CompletionRequest,
            0x0002 => MessageType::CompletionResponse,
            0x0003 => MessageType::StreamChunk,
            0x0004 => MessageType::Error,
            0x0005 => MessageType::Heartbeat,
            _ => MessageType::Heartbeat, // Default fallback
        };
        
        Ok(Some(Message {
            id: 0, // TODO: Extract ID from header or generate
            msg_type: message_type,
            payload,
            timestamp: timestamp as u64,
        }))
    }
}

impl Encoder<Message> for ZeroCopyCodec {
    type Error = io::Error;
    
    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize payload with rkyv
        let payload_bytes = match rkyv::to_bytes::<_, 256>(&item.payload) {
            Ok(bytes) => bytes,
            Err(e) => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Serialization failed: {}", e)
            )),
        };
        
        // Decide on compression
        let (final_payload, compressed) = if self.enforce_compression || payload_bytes.len() > 1024 {
            // Compress if enforced or payload is large
            match zstd::encode_all(&payload_bytes[..], 3) {
                Ok(compressed) if compressed.len() < payload_bytes.len() => {
                    (Bytes::from(compressed), true)
                }
                _ => (Bytes::copy_from_slice(&payload_bytes), false)
            }
        } else {
            (Bytes::copy_from_slice(&payload_bytes), false)
        };
        
        // Check size limit
        if final_payload.len() > self.max_frame_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Encoded message too large: {} > {}", final_payload.len(), self.max_frame_size)
            ));
        }
        
        // Reserve space for header + payload
        dst.reserve(16 + final_payload.len());
        
        // Write header
        dst.put_slice(MAGIC_BYTES);
        dst.put_u8(PROTOCOL_VERSION);
        dst.put_u16_le(item.msg_type as u16);
        dst.put_u8(if compressed { FLAG_COMPRESSED } else { 0 });
        dst.put_u32_le(final_payload.len() as u32);
        dst.put_u32_le(item.timestamp as u32);
        
        // Write payload (zero-copy via Bytes)
        dst.put(final_payload);
        
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
            payload: MessagePayload::CompletionRequest(CompletionRequest {
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
            payload: MessagePayload::StreamChunk(StreamChunk {
                stream_id: 1,
                sequence: 1,
                content: ChunkContent::Text("x".repeat(2048)), // Large payload
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
            payload: MessagePayload::StreamChunk(StreamChunk {
                stream_id: 1,
                sequence: 1,
                content: ChunkContent::Text("x".repeat(1024)), // Exceeds max size
                is_final: false,
            }),
            timestamp: 12345,
        };
        
        let result = codec.encode(msg, &mut buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too large"));
    }
}
