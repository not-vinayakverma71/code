//! Property-based tests for compact structures
//! Tests invariants that should hold for all inputs

use lapce_tree_sitter::compact::{DeltaEncoder, DeltaDecoder};
use lapce_tree_sitter::compact::bytecode::BytecodeReader;

/// Property: Delta encoding preserves order and values
#[test]
fn prop_delta_encoding_preserves_order() {
    let sequences = vec![
        vec![10, 20, 30, 40, 50],
        vec![0, 1, 2, 3, 4, 5],
        vec![100, 200, 300],
        vec![1000, 1005, 1010, 1015],
        vec![0],  // Single value
        vec![],   // Empty sequence
    ];
    
    for sequence in sequences {
        let mut encoder = DeltaEncoder::new();
        for &val in &sequence {
            encoder.encode(val);
        }
        let encoded = encoder.finish();
        
        let mut decoder = DeltaDecoder::new(&encoded);
        let mut decoded = Vec::new();
        while decoder.has_more() {
            decoded.push(decoder.decode().unwrap());
        }
        
        assert_eq!(
            sequence, decoded,
            "Delta encoding should preserve sequence {:?}", sequence
        );
    }
}

/// Property: Delta encoding is space-efficient for clustered values
#[test]
fn prop_delta_encoding_compression() {
    let clustered = vec![1000, 1001, 1002, 1003, 1004, 1005];
    
    let mut encoder = DeltaEncoder::new();
    for &val in &clustered {
        encoder.encode(val);
    }
    let encoded = encoder.finish();
    
    let original_size = clustered.len() * 8;
    let encoded_size = encoded.len();
    
    let compression_ratio = (original_size - encoded_size) as f64 / original_size as f64;
    assert!(
        compression_ratio > 0.5,
        "Delta encoding should compress clustered data by >50%, got {:.1}%",
        compression_ratio * 100.0
    );
}

/// Property: Delta encoding handles large gaps
#[test]
fn prop_delta_encoding_large_gaps() {
    let sparse = vec![0, 1_000_000, 2_000_000, 3_000_000];
    
    let mut encoder = DeltaEncoder::new();
    for &val in &sparse {
        encoder.encode(val);
    }
    let encoded = encoder.finish();
    
    let mut decoder = DeltaDecoder::new(&encoded);
    let mut decoded = Vec::new();
    while decoder.has_more() {
        decoded.push(decoder.decode().unwrap());
    }
    
    assert_eq!(sparse, decoded, "Should handle large gaps correctly");
}

/// Property: Empty encoding produces empty output
#[test]
fn prop_delta_encoding_empty() {
    let encoder = DeltaEncoder::new();
    let encoded = encoder.finish();
    
    let mut decoder = DeltaDecoder::new(&encoded);
    assert!(!decoder.has_more(), "Empty encoding should decode to nothing");
}

/// Property: Monotonically increasing sequences
#[test]
fn prop_delta_encoding_monotonic() {
    let mut values = Vec::new();
    let mut current = 0u64;
    for i in 0..100 {
        current += (i * 7 + 1) as u64;
        values.push(current);
    }
    
    let mut encoder = DeltaEncoder::new();
    for &val in &values {
        encoder.encode(val);
    }
    let encoded = encoder.finish();
    
    let mut decoder = DeltaDecoder::new(&encoded);
    let mut decoded = Vec::new();
    while decoder.has_more() {
        decoded.push(decoder.decode().unwrap());
    }
    
    assert_eq!(values, decoded, "Should preserve monotonic sequence");
    
    for i in 1..decoded.len() {
        assert!(
            decoded[i] >= decoded[i-1],
            "Decoded sequence should be monotonic"
        );
    }
}

/// Property: Varint encoding range coverage
#[test]
fn prop_varint_all_ranges() {
    let test_cases = vec![
        (0u64, "zero"),
        (127u64, "max-1-byte"),
        (128u64, "min-2-byte"),
        (16383u64, "max-2-byte"),
        (16384u64, "min-3-byte"),
        (2097151u64, "max-3-byte"),
        (u32::MAX as u64, "u32-max"),
        (u64::MAX / 2, "large-value"),
    ];
    
    for (value, label) in test_cases {
        let mut encoded = Vec::new();
        let mut val = value;
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            encoded.push(byte);
            if val == 0 {
                break;
            }
        }
        
        let mut reader = BytecodeReader::new(&encoded);
        let decoded = reader.read_varint();
        
        assert_eq!(
            decoded, Some(value),
            "Varint roundtrip failed for {} (value={})", label, value
        );
    }
}

/// Property: Delta encoding doesn't lose precision
#[test]
fn prop_delta_no_precision_loss() {
    let boundaries = vec![
        u64::MIN,
        u64::MAX / 2,
        u64::MAX / 4,
        u64::MAX / 8,
    ];
    
    for &val in &boundaries {
        let mut encoder = DeltaEncoder::new();
        encoder.encode(val);
        let encoded = encoder.finish();
        
        let mut decoder = DeltaDecoder::new(&encoded);
        let decoded = decoder.decode().unwrap();
        
        assert_eq!(val, decoded, "Precision lost for value {}", val);
    }
}
