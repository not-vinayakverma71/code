/// Ultra-simple test to check buffer creation/open
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;

#[test]
fn test_buffer_create_open() {
    let path = format!("/tmp/test_buf_{}.shm", std::process::id());
    let _ = std::fs::remove_file(&path);
    
    println!("1. Creating buffer...");
    let mut write_buf = SharedMemoryBuffer::create(&path, 4096).expect("Failed to create");
    println!("   ✓ Buffer created");
    
    println!("2. Opening buffer...");
    let mut read_buf = SharedMemoryBuffer::open(&path, 4096).expect("Failed to open");
    println!("   ✓ Buffer opened");
    
    println!("3. Writing data...");
    write_buf.write(b"hello").expect("Failed to write");
    println!("   ✓ Data written");
    
    println!("4. Reading data...");
    if let Some(data) = read_buf.read() {
        let s = String::from_utf8_lossy(&data);
        println!("   ✓ Read: {}", s);
        assert_eq!(&data, b"hello");
    } else {
        panic!("No data read");
    }
    
    println!("\n✅ Buffer test passed!");
    let _ = std::fs::remove_file(&path);
}
