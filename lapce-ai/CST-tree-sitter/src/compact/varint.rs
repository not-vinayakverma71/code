//! Variable-length integer encoding for efficient storage of values with varying magnitudes
//! Optimized for delta-encoded sequences common in CST position data

use std::io::{self};

/// Variable-length integer encoder/decoder using LEB128
pub struct VarInt;

impl VarInt {
    /// Encode a single u64 value to LEB128
    #[inline]
    pub fn encode_u64(value: u64, output: &mut Vec<u8>) {
        let mut value = value;
        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                output.push(byte | 0x80);
            } else {
                output.push(byte);
                break;
            }
        }
    }
    
    /// Decode a single u64 value from LEB128
    #[inline]
    pub fn decode_u64(input: &[u8]) -> io::Result<(u64, usize)> {
        let mut value = 0u64;
        let mut shift = 0;
        let mut consumed = 0;
        
        for &byte in input {
            consumed += 1;
            
            if shift >= 64 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "VarInt overflow"));
            }
            
            value |= ((byte & 0x7F) as u64) << shift;
            
            if byte & 0x80 == 0 {
                return Ok((value, consumed));
            }
            
            shift += 7;
        }
        
        Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Incomplete VarInt"))
    }
    
    /// Encode a signed i64 value using zigzag encoding + LEB128
    #[inline]
    pub fn encode_i64(value: i64, output: &mut Vec<u8>) {
        let zigzag = ((value << 1) ^ (value >> 63)) as u64;
        Self::encode_u64(zigzag, output);
    }
    
    /// Decode a signed i64 value from zigzag + LEB128
    #[inline]
    pub fn decode_i64(input: &[u8]) -> io::Result<(i64, usize)> {
        let (zigzag, consumed) = Self::decode_u64(input)?;
        let value = ((zigzag >> 1) as i64) ^ -((zigzag & 1) as i64);
        Ok((value, consumed))
    }
    
    /// Calculate encoded size for u64
    pub fn size_u64(value: u64) -> usize {
        if value == 0 {
            return 1;
        }
        let bits = 64 - value.leading_zeros() as usize;
        (bits + 6) / 7
    }
}

/// Delta encoder for monotone sequences
pub struct DeltaEncoder {
    last_value: u64,
    output: Vec<u8>,
}

impl DeltaEncoder {
    /// Create new delta encoder
    pub fn new() -> Self {
        Self {
            last_value: 0,
            output: Vec::new(),
        }
    }
    
    /// Create with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            last_value: 0,
            output: Vec::with_capacity(capacity),
        }
    }
    
    /// Encode next value (must be >= last value for unsigned deltas)
    pub fn encode(&mut self, value: u64) {
        assert!(value >= self.last_value, "Values must be non-decreasing");
        let delta = value - self.last_value;
        VarInt::encode_u64(delta, &mut self.output);
        self.last_value = value;
    }
    
    /// Encode a batch of values
    pub fn encode_batch(&mut self, values: &[u64]) {
        for &value in values {
            self.encode(value);
        }
    }
    
    /// Get encoded bytes
    pub fn finish(self) -> Vec<u8> {
        self.output
    }
    
    /// Get encoded bytes without consuming
    pub fn bytes(&self) -> &[u8] {
        &self.output
    }
    
    /// Reset encoder
    pub fn reset(&mut self) {
        self.last_value = 0;
        self.output.clear();
    }
}

/// Delta decoder for monotone sequences
pub struct DeltaDecoder<'a> {
    input: &'a [u8],
    position: usize,
    last_value: u64,
}

impl<'a> DeltaDecoder<'a> {
    /// Create new delta decoder
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            position: 0,
            last_value: 0,
        }
    }
    
    /// Decode next value
    pub fn decode(&mut self) -> io::Result<u64> {
        let (delta, consumed) = VarInt::decode_u64(&self.input[self.position..])?;
        self.position += consumed;
        self.last_value += delta;
        Ok(self.last_value)
    }
    
    /// Decode all remaining values
    pub fn decode_all(&mut self) -> io::Result<Vec<u64>> {
        let mut values = Vec::new();
        while self.position < self.input.len() {
            values.push(self.decode()?);
        }
        Ok(values)
    }
    
    /// Check if more values available
    pub fn has_more(&self) -> bool {
        self.position < self.input.len()
    }
    
    /// Reset decoder
    pub fn reset(&mut self) {
        self.position = 0;
        self.last_value = 0;
    }
}

/// Prefix sum index for O(1) random access to delta-encoded sequences
#[derive(Clone)]
pub struct PrefixSumIndex {
    /// Block size (typically 256)
    block_size: usize,
    
    /// Prefix sums at block boundaries
    block_sums: Vec<u64>,
    
    /// Delta-encoded data
    data: Vec<u8>,
}

impl PrefixSumIndex {
    /// Build index from values
    pub fn from_values(values: &[u64], block_size: usize) -> Self {
        let mut encoder = DeltaEncoder::new();
        let mut block_sums = Vec::new();
        
        for (i, &value) in values.iter().enumerate() {
            if i % block_size == 0 {
                block_sums.push(value);
                encoder.last_value = value; // Reset delta base
            } else {
                // Only encode non-block-boundary values
                encoder.encode(value);
            }
        }
        
        Self {
            block_size,
            block_sums,
            data: encoder.finish(),
        }
    }
    
    /// Get value at index with O(1) + small scan
    pub fn get(&self, index: usize) -> io::Result<u64> {
        let block_idx = index / self.block_size;
        let block_offset = index % self.block_size;
        
        if block_idx >= self.block_sums.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of bounds"));
        }
        
        // Start from block boundary
        let mut decoder = DeltaDecoder::new(&self.data);
        decoder.last_value = self.block_sums[block_idx];
        
        // Skip to correct position in data
        let mut data_position = 0;
        for i in 0..block_idx {
            // Skip entire blocks (block_size - 1 values since we don't encode block boundaries)
            for _ in 0..(self.block_size - 1) {
                let (_, consumed) = VarInt::decode_u64(&self.data[data_position..])?;
                data_position += consumed;
            }
        }
        decoder.position = data_position;
        
        // Decode up to target index
        // If block_offset is 0, we want the block boundary value itself
        if block_offset == 0 {
            return Ok(decoder.last_value);
        }
        
        // Decode block_offset times to get to the target
        let mut value = decoder.last_value;
        for _ in 0..block_offset {
            value = decoder.decode()?;
        }
        
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_u64() {
        let test_cases = vec![
            0u64,
            127,
            128,
            16383,
            16384,
            u32::MAX as u64,
            u64::MAX,
        ];
        
        for value in test_cases {
            let mut encoded = Vec::new();
            VarInt::encode_u64(value, &mut encoded);
            
            let (decoded, consumed) = VarInt::decode_u64(&encoded).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(consumed, encoded.len());
            assert_eq!(VarInt::size_u64(value), encoded.len());
        }
    }

    #[test]
    fn test_varint_i64() {
        let test_cases = vec![
            0i64,
            -1,
            1,
            -128,
            127,
            i32::MIN as i64,
            i32::MAX as i64,
            i64::MIN,
            i64::MAX,
        ];
        
        for value in test_cases {
            let mut encoded = Vec::new();
            VarInt::encode_i64(value, &mut encoded);
            
            let (decoded, consumed) = VarInt::decode_i64(&encoded).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(consumed, encoded.len());
        }
    }

    #[test]
    fn test_delta_encoder() {
        let values = vec![0, 10, 15, 16, 30, 100, 101, 200];
        
        let mut encoder = DeltaEncoder::new();
        encoder.encode_batch(&values);
        let encoded = encoder.finish();
        
        let mut decoder = DeltaDecoder::new(&encoded);
        let decoded = decoder.decode_all().unwrap();
        
        assert_eq!(decoded, values);
    }

    #[test]
    fn test_delta_monotone_positions() {
        // Simulate CST start positions
        let positions: Vec<u64> = vec![
            0, 10, 15, 25, 30, 45, 50, 100, 150, 200,
            250, 300, 350, 400, 450, 500, 600, 700, 800, 900,
        ];
        
        let mut encoder = DeltaEncoder::new();
        encoder.encode_batch(&positions);
        let encoded = encoder.finish();
        
        // Check compression ratio
        let original_size = positions.len() * 8; // 8 bytes per u64
        let compressed_size = encoded.len();
        let ratio = original_size as f64 / compressed_size as f64;
        
        assert!(ratio > 5.0, "Compression ratio {} is too low", ratio);
        
        // Verify decoding
        let mut decoder = DeltaDecoder::new(&encoded);
        for &expected in &positions {
            assert_eq!(decoder.decode().unwrap(), expected);
        }
    }

    #[test]
    fn test_prefix_sum_index() {
        let values: Vec<u64> = (0..1000).map(|i| i * 10).collect();
        
        let index = PrefixSumIndex::from_values(&values, 100);
        
        // Test random access
        assert_eq!(index.get(0).unwrap(), 0);
        assert_eq!(index.get(50).unwrap(), 500);
        assert_eq!(index.get(99).unwrap(), 990);
        assert_eq!(index.get(100).unwrap(), 1000);
        assert_eq!(index.get(150).unwrap(), 1500);
        assert_eq!(index.get(999).unwrap(), 9990);
    }

    #[test]
    fn test_real_cst_positions() {
        // Simulate real CST node positions (start bytes)
        let mut positions = Vec::new();
        let mut pos = 0u64;
        
        for _ in 0..10000 {
            positions.push(pos);
            pos += (pos % 100) + 1; // Variable increments
        }
        
        let mut encoder = DeltaEncoder::new();
        encoder.encode_batch(&positions);
        let encoded = encoder.finish();
        
        let bytes_per_position = encoded.len() as f64 / positions.len() as f64;
        assert!(bytes_per_position < 2.5, 
                "Too many bytes per position: {:.2}", bytes_per_position);
        
        // Verify
        let mut decoder = DeltaDecoder::new(&encoded);
        let decoded = decoder.decode_all().unwrap();
        assert_eq!(decoded, positions);
    }

    #[test]
    fn test_size_calculation() {
        assert_eq!(VarInt::size_u64(0), 1);
        assert_eq!(VarInt::size_u64(127), 1);
        assert_eq!(VarInt::size_u64(128), 2);
        assert_eq!(VarInt::size_u64(16383), 2);
        assert_eq!(VarInt::size_u64(16384), 3);
        assert_eq!(VarInt::size_u64(u32::MAX as u64), 5);
        assert_eq!(VarInt::size_u64(u64::MAX), 10);
    }
}
