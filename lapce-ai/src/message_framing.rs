/// Message Framing for IPC - handles proper message boundaries
/// TypeScript node-ipc uses JSON messages with delimiters
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json;
use std::io;

const MESSAGE_DELIMITER: &[u8] = b"\x0C"; // Form feed character used by node-ipc

/// Frame a message with delimiter (matches node-ipc format)
pub fn frame_message(data: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(data.len() + MESSAGE_DELIMITER.len());
    framed.extend_from_slice(data);
    framed.extend_from_slice(MESSAGE_DELIMITER);
    framed
}

/// Read a delimited message from stream
pub async fn read_delimited_message<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    buffer: &mut Vec<u8>,
) -> io::Result<Option<Vec<u8>>> {
    let mut temp_buffer = vec![0u8; 4096];
    
    loop {
        // Check for delimiter in existing buffer first
        if let Some(pos) = buffer.windows(MESSAGE_DELIMITER.len())
            .position(|window| window == MESSAGE_DELIMITER) 
        {
            // Found a complete message
            let message = buffer[..pos].to_vec();
            // Remove message + delimiter from buffer
            buffer.drain(..pos + MESSAGE_DELIMITER.len());
            return Ok(Some(message));
        }
        
        // No complete message yet, read more data
        let n = reader.read(&mut temp_buffer).await?;
        if n == 0 {
            // Connection closed
            if buffer.is_empty() {
                return Ok(None);
            } else {
                // Return partial data if any
                let message = buffer.clone();
                buffer.clear();
                return Ok(Some(message));
            }
        }
        
        buffer.extend_from_slice(&temp_buffer[..n]);
    }
}

/// Write a framed message to stream
pub async fn write_framed_message<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    message: &impl serde::Serialize,
) -> io::Result<()> {
    let json = serde_json::to_vec(message)?;
    let framed = frame_message(&json);
    writer.write_all(&framed).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_frame_message() {
        let data = b"test message";
        let framed = frame_message(data);
        assert_eq!(&framed[..data.len()], data);
        assert_eq!(&framed[data.len()..], MESSAGE_DELIMITER);
    }
    
    #[tokio::test]
    async fn test_read_delimited() {
        let mut buffer = Vec::new();
        let input = b"message1\x0cmessage2\x0c";
        let mut cursor = io::Cursor::new(input);
        
        let msg1 = read_delimited_message(&mut cursor, &mut buffer).await.unwrap();
        assert_eq!(msg1, Some(b"message1".to_vec()));
        
        let msg2 = read_delimited_message(&mut cursor, &mut buffer).await.unwrap();
        assert_eq!(msg2, Some(b"message2".to_vec()));
    }
}
