use bytes::{Bytes, BytesMut};

fn main() {
    let mut buffer = BytesMut::with_capacity(8192);
    let input = b"data: {\"text\":\"Hello\"}\n\ndata: {\"text\":\"World\"}\n\ndata: [DONE]\n\n";
    buffer.extend_from_slice(input);
    
    println!("Buffer contents: {:?}", std::str::from_utf8(&buffer));
    println!("Buffer length: {}", buffer.len());
    
    // Find first newline
    let pos1 = buffer.iter().position(|&b| b == b'\n');
    println!("First newline at: {:?}", pos1);
    
    // Check what parse_next_event would do
    if let Some(line_end) = buffer.iter().position(|&b| b == b'\n') {
        let line = &buffer[..line_end];
        println!("First line: {:?}", std::str::from_utf8(line));
        println!("Line empty? {}", line.is_empty());
    }
}
