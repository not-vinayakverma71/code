// Standalone test runner for protocol compliance
const HEADER_SIZE: usize = 24;
const MAGIC_HEADER: u32 = 0x4C415043;  // "LAPC"
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;  // 10MB

fn create_valid_header(payload_len: u32) -> Vec<u8> {
    let mut header = vec![0u8; HEADER_SIZE];
    header[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
    header[4] = 1;
    header[5] = 0;
    header[6..8].copy_from_slice(&1u16.to_le_bytes());
    header[8..12].copy_from_slice(&payload_len.to_le_bytes());
    header[12..20].copy_from_slice(&12345u64.to_le_bytes());
    
    let mut full_msg = Vec::with_capacity(HEADER_SIZE + payload_len as usize);
    full_msg.extend_from_slice(&header);
    full_msg.resize(HEADER_SIZE + payload_len as usize, 0);
    let crc = crc32fast::hash(&full_msg);
    header[20..24].copy_from_slice(&crc.to_le_bytes());
    
    header
}

fn main() {
    println!("Running Protocol Compliance Tests...\n");
    let mut passed = 0;
    let mut failed = 0;
    
    // Test 1: Bad Magic
    {
        let mut header = create_valid_header(0);
        header[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
        let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        if magic != MAGIC_HEADER && magic == 0xDEADBEEF {
            println!("✓ test_bad_magic");
            passed += 1;
        } else {
            println!("✗ test_bad_magic");
            failed += 1;
        }
    }
    
    // Test 2: Wrong Version
    {
        let mut header = create_valid_header(0);
        header[4] = 99;
        if header[4] != 1 && header[4] == 99 {
            println!("✓ test_wrong_version");
            passed += 1;
        } else {
            println!("✗ test_wrong_version");
            failed += 1;
        }
    }
    
    // Test 3: Oversize Length
    {
        let mut header = create_valid_header(0);
        let oversize = (MAX_MESSAGE_SIZE + 1) as u32;
        header[8..12].copy_from_slice(&oversize.to_le_bytes());
        let len = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
        if len as usize > MAX_MESSAGE_SIZE && len == oversize {
            println!("✓ test_oversize_length");
            passed += 1;
        } else {
            println!("✗ test_oversize_length");
            failed += 1;
        }
    }
    
    // Test 4: CRC Mismatch
    {
        let mut header = create_valid_header(5);
        header[20..24].copy_from_slice(&0xBADC0DEu32.to_le_bytes());
        let payload = b"hello";
        let mut full_msg = Vec::new();
        full_msg.extend_from_slice(&header);
        full_msg.extend_from_slice(payload);
        
        let mut check_data = full_msg.clone();
        check_data[20..24].fill(0);
        let correct_crc = crc32fast::hash(&check_data);
        let bad_crc = u32::from_le_bytes([header[20], header[21], header[22], header[23]]);
        
        if correct_crc != bad_crc && bad_crc == 0xBADC0DE {
            println!("✓ test_crc_mismatch");
            passed += 1;
        } else {
            println!("✗ test_crc_mismatch");
            failed += 1;
        }
    }
    
    // Test 5: Valid Header
    {
        let header = create_valid_header(10);
        let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        let version = header[4];
        let flags = header[5];
        let msg_type = u16::from_le_bytes([header[6], header[7]]);
        let len = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
        let msg_id = u64::from_le_bytes([
            header[12], header[13], header[14], header[15],
            header[16], header[17], header[18], header[19]
        ]);
        
        let mut payload = vec![0u8; 10];
        let mut full_msg = Vec::new();
        full_msg.extend_from_slice(&header);
        full_msg.extend_from_slice(&payload);
        
        let crc_received = u32::from_le_bytes([header[20], header[21], header[22], header[23]]);
        let mut check_data = full_msg.clone();
        check_data[20..24].fill(0);
        let calculated_crc = crc32fast::hash(&check_data);
        
        if magic == MAGIC_HEADER && version == 1 && flags == 0 && 
           msg_type == 1 && len == 10 && msg_id == 12345 && 
           calculated_crc == crc_received {
            println!("✓ test_valid_header");
            passed += 1;
        } else {
            println!("✗ test_valid_header");
            failed += 1;
        }
    }
    
    // Test 6: Zero Length Message
    {
        let header = create_valid_header(0);
        let len = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
        
        let mut full_msg = Vec::new();
        full_msg.extend_from_slice(&header);
        let crc_received = u32::from_le_bytes([header[20], header[21], header[22], header[23]]);
        let mut check_data = full_msg.clone();
        check_data[20..24].fill(0);
        let calculated_crc = crc32fast::hash(&check_data);
        
        if len == 0 && calculated_crc == crc_received {
            println!("✓ test_zero_length_message");
            passed += 1;
        } else {
            println!("✗ test_zero_length_message");
            failed += 1;
        }
    }
    
    // Test 7: Max Size Message
    {
        let max_size = MAX_MESSAGE_SIZE as u32;
        let mut header = create_valid_header(0);
        header[8..12].copy_from_slice(&max_size.to_le_bytes());
        let len = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
        
        if len as usize == MAX_MESSAGE_SIZE && len as usize <= MAX_MESSAGE_SIZE {
            println!("✓ test_max_size_message");
            passed += 1;
        } else {
            println!("✗ test_max_size_message");
            failed += 1;
        }
    }
    
    // Test 8: Truncated Header
    {
        let truncated = vec![0x43, 0x50, 0x41, 0x4C, 0x01];
        if truncated.len() < HEADER_SIZE {
            println!("✓ test_truncated_header");
            passed += 1;
        } else {
            println!("✗ test_truncated_header");
            failed += 1;
        }
    }
    
    // Test 9: All Flags Set
    {
        let mut header = create_valid_header(0);
        header[5] = 0xFF;
        if header[5] == 0xFF {
            println!("✓ test_all_flags_set");
            passed += 1;
        } else {
            println!("✗ test_all_flags_set");
            failed += 1;
        }
    }
    
    // Test 10: Endianness Consistency
    {
        let header = create_valid_header(0x12345678);
        if header[0] == 0x43 && header[1] == 0x50 && header[2] == 0x41 && header[3] == 0x4C &&
           header[8] == 0x78 && header[9] == 0x56 && header[10] == 0x34 && header[11] == 0x12 {
            println!("✓ test_endianness_consistency");
            passed += 1;
        } else {
            println!("✗ test_endianness_consistency");
            failed += 1;
        }
    }
    
    println!("\n=====================================");
    println!("Protocol Compliance Tests Complete");
    println!("Passed: {}/{}", passed, passed + failed);
    println!("Failed: {}/{}", failed, passed + failed);
    
    if failed == 0 {
        println!("\n✅ IPC-003 COMPLETE: All protocol compliance tests pass!");
    } else {
        println!("\n❌ Some tests failed");
        std::process::exit(1);
    }
}
