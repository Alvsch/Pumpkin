use std::io::{Read, Write};

use enum_dispatch::enum_dispatch;
use flate2::bufread::{GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder};
use lz4_java_wrc::{Context, Lz4BlockInput, Lz4BlockOutput};

#[enum_dispatch(Compression)]
pub trait CompressionAlgorithm {
    fn compress(&self, uncompressed_data: &[u8], compression_level: u32) -> Vec<u8>;
    fn decompress(&self, compressed_data: &[u8]) -> Vec<u8>;
}

#[enum_dispatch]
pub enum Compression {
    Gzip,
    Zlib,
    NoCompression,
    Lz4,
    Custom(Box<dyn CompressionAlgorithm>),
}

pub struct Gzip;
pub struct Zlib;
pub struct NoCompression;
pub struct Lz4;

impl CompressionAlgorithm for Gzip {
    fn compress(&self, uncompressed_data: &[u8], compression_level: u32) -> Vec<u8> {
        let mut encoder = GzEncoder::new(
            uncompressed_data,
            flate2::Compression::new(compression_level),
        );
        let mut buf = Vec::new();
        encoder.read_to_end(&mut buf).unwrap();
        buf
    }

    fn decompress(&self, compressed_data: &[u8]) -> Vec<u8> {
        let mut decoder = GzDecoder::new(compressed_data);
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf).unwrap();
        buf
    }
}

impl CompressionAlgorithm for Zlib {
    fn compress(&self, uncompressed_data: &[u8], compression_level: u32) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(
            uncompressed_data,
            flate2::Compression::new(compression_level),
        );
        let mut buf = Vec::new();
        encoder.read_to_end(&mut buf).unwrap();
        buf
    }

    fn decompress(&self, compressed_data: &[u8]) -> Vec<u8> {
        let mut decoder = ZlibDecoder::new(compressed_data);
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf).unwrap();
        buf
    }
}

impl CompressionAlgorithm for NoCompression {
    fn compress(&self, uncompressed_data: &[u8], _compression_level: u32) -> Vec<u8> {
        uncompressed_data.to_vec()
    }

    fn decompress(&self, compressed_data: &[u8]) -> Vec<u8> {
        compressed_data.to_vec()
    }
}

impl CompressionAlgorithm for Lz4 {
    fn compress(&self, uncompressed_data: &[u8], compression_level: u32) -> Vec<u8> {
        const LZ4_COMPRESSION_LEVEL_BASE: u32 = 10;

        let block_size = 1 << (LZ4_COMPRESSION_LEVEL_BASE + compression_level);
        let mut buf = Vec::new();
        {
            let mut block =
                Lz4BlockOutput::with_context(&mut buf, Context::default(), block_size).unwrap();
            block.write_all(uncompressed_data).unwrap();
            block.flush().unwrap();
        }
        buf
    }

    fn decompress(&self, compressed_data: &[u8]) -> Vec<u8> {
        let mut block = Lz4BlockInput::new(compressed_data);
        let mut buf = Vec::new();
        block.read_to_end(&mut buf).unwrap();
        buf
    }
}

impl Compression {
    const GZIP_ID: u8 = 1;
    const ZLIB_ID: u8 = 2;
    const NO_COMPRESSION_ID: u8 = 3;
    const LZ4_ID: u8 = 4;
    const CUSTOM_ID: u8 = 127;

    /// Returns Ok when a compression is found otherwise an Err
    #[expect(clippy::result_unit_err)]
    pub fn from_u8(byte: u8) -> Result<Self, ()> {
        match byte {
            Self::GZIP_ID => Ok(Gzip.into()),
            Self::ZLIB_ID => Ok(Zlib.into()),
            // Uncompressed (since a version before 1.15.1)
            Self::NO_COMPRESSION_ID => Ok(NoCompression.into()),
            Self::LZ4_ID => Ok(Lz4.into()),
            Self::CUSTOM_ID => todo!(),
            // Unknown format
            _ => Err(()),
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            Compression::Gzip(_) => Self::GZIP_ID,
            Compression::Zlib(_) => Self::ZLIB_ID,
            Compression::NoCompression(_) => Self::NO_COMPRESSION_ID,
            Compression::Lz4(_) => Self::LZ4_ID,
            Compression::Custom(_) => Self::CUSTOM_ID,
        }
    }

}

impl CompressionAlgorithm for Box<dyn CompressionAlgorithm> {
    fn compress(&self, uncompressed_data: &[u8], compression_level: u32) -> Vec<u8> {
        (**self).compress(uncompressed_data, compression_level)
    }

    fn decompress(&self, compressed_data: &[u8]) -> Vec<u8> {
        (**self).decompress(compressed_data)
    }
}

impl From<pumpkin_config::chunk::Compression> for Compression {
    fn from(value: pumpkin_config::chunk::Compression) -> Self {
        // :c
        match value {
            pumpkin_config::chunk::Compression::GZip => Gzip.into(),
            pumpkin_config::chunk::Compression::ZLib => Zlib.into(),
            pumpkin_config::chunk::Compression::LZ4 => Lz4.into(),
            pumpkin_config::chunk::Compression::Custom => todo!(),
        }
    }
}
