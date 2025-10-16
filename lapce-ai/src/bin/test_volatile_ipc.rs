/// Test cross-process volatile ring buffer visibility
/// Writer process writes data, reader process should see it immediately

use std::process::Command;
use std::time::Duration;
use lapce_ai_rust::ipc::shm_buffer_volatile::VolatileSharedMemoryBuffer;

const TEST_SHM_PATH: &str = "/tmp/test_volatile_ring";
const RING_CAPACITY: u32 = 4096;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <writer|reader>", args[0]);
        std::process::exit(1);
    }
    
    match args[1].as_str() {
        "writer" => run_writer(),
        "reader" => run_reader(),
        "test" => run_full_test(),
        _ => {
            eprintln!("Invalid mode: {}", args[1]);
            eprintln!("Use: writer, reader, or test");
            std::process::exit(1);
        }
    }
}

fn run_full_test() {
    println!("\n═══ Testing Cross-Process Volatile Ring Buffer ═══\n");
    
    // Clean up any stale SHM
    let _ = std::fs::remove_file(format!("{}", TEST_SHM_PATH));
    
    // Spawn writer process
    println!("[TEST] Spawning writer process...");
    let mut writer = Command::new(std::env::current_exe().unwrap())
        .arg("writer")
        .spawn()
        .expect("Failed to spawn writer");
    
    // Give writer time to create SHM and write data
    std::thread::sleep(Duration::from_millis(500));
    
    // Spawn reader process
    println!("[TEST] Spawning reader process...");
    let reader_output = Command::new(std::env::current_exe().unwrap())
        .arg("reader")
        .output()
        .expect("Failed to spawn reader");
    
    // Wait for writer
    let _ = writer.wait();
    
    // Check results
    let reader_stdout = String::from_utf8_lossy(&reader_output.stdout);
    println!("\n[TEST] Reader output:\n{}", reader_stdout);
    
    if reader_stdout.contains("✓ Read 27 bytes") && reader_stdout.contains("Hello from volatile writer!") {
        println!("\n✅ SUCCESS: Cross-process volatile ring buffer works!");
        println!("   Writer's data was visible to reader process");
    } else {
        println!("\n❌ FAILED: Reader did not see writer's data");
        println!("   This indicates cross-process visibility issue");
        std::process::exit(1);
    }
}

fn run_writer() {
    println!("[WRITER] Creating shared memory buffer...");
    
    let buffer = VolatileSharedMemoryBuffer::create(TEST_SHM_PATH, RING_CAPACITY)
        .expect("Failed to create SHM buffer");
    
    println!("[WRITER] Buffer created");
    
    // Write test message
    let message = b"Hello from volatile writer!";
    println!("[WRITER] Writing: {:?}", std::str::from_utf8(message).unwrap());
    
    buffer.write(message).expect("Failed to write");
    
    println!("[WRITER] Write complete. Header state:");
    let header = buffer.header();
    println!("  write_pos: {}", header.load_write_pos());
    println!("  read_pos: {}", header.load_read_pos());
    println!("  available_read: {}", header.available_read());
    
    // Keep buffer alive for reader to open
    std::thread::sleep(Duration::from_secs(2));
    
    println!("[WRITER] Exiting");
}

fn run_reader() {
    println!("[READER] Opening shared memory buffer...");
    
    let buffer = VolatileSharedMemoryBuffer::open(TEST_SHM_PATH, RING_CAPACITY)
        .expect("Failed to open SHM buffer");
    
    println!("[READER] Buffer opened. Header state:");
    let header = buffer.header();
    println!("  write_pos: {}", header.load_write_pos_acquire());
    println!("  read_pos: {}", header.load_read_pos());
    println!("  available_read: {}", header.available_read());
    
    // Try to read
    let mut data = Vec::new();
    let n = buffer.read(&mut data, 1024).expect("Failed to read");
    
    if n > 0 {
        println!("[READER] ✓ Read {} bytes: {:?}", n, std::str::from_utf8(&data).unwrap());
    } else {
        println!("[READER] ✗ Read 0 bytes - writer data not visible!");
        std::process::exit(1);
    }
}
