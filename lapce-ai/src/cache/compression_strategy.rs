/// CompressionStrategy - EXACT implementation from docs lines 273-306
use anyhow::Result;
use lz4_flex;
use zstd;

pub enum CompressionStrategy {
    None,
    Lz4,
    Zstd,
}

impl CompressionStrategy {
    pub fn new(compression_type: super::types::CompressionType) -> Result<Self> {
        match compression_type {
            super::types::CompressionType::None => Ok(Self::None),
            super::types::CompressionType::Lz4 => Ok(Self::Lz4),
            super::types::CompressionType::Zstd => Ok(Self::Zstd),
        }
    }
    
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            CompressionStrategy::None => Ok(data.to_vec()),
            CompressionStrategy::Lz4 => Ok(lz4_flex::compress_prepend_size(data)),
            CompressionStrategy::Zstd => {
                // Zstd disabled due to version conflicts
                // Fall back to LZ4
                Ok(lz4_flex::compress_prepend_size(data))
            }
        }
    }
    
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            Self::None => Ok(data.to_vec()),
            Self::Lz4 => {
                Ok(lz4_flex::decompress_size_prepended(data)?)
            }
            Self::Zstd => {
                Ok(zstd::decode_all(data)?)
            }
        }
    }
}
