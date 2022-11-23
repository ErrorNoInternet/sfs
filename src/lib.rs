use std::fmt;
use xxhash_rust::xxh3::Xxh3;
#[macro_use]
extern crate structure;

pub const SFS_FORMAT_VERSION: u8 = 1;
pub const SFS_VERSION_STRING: &str = "1.0.0";

#[derive(Debug, Default, Clone)]
pub struct FileMetadata {
    pub format_version: u8,
    pub hashing_algorithm: u8,
    pub checksum: u64,
    pub total_bytes: u64,
    pub chunk_size: u64,
}
impl FileMetadata {
    pub fn pack(&self) -> Vec<u8> {
        let metadata_structure = structure!("BBQQQ");
        metadata_structure
            .pack(
                self.format_version,
                self.hashing_algorithm,
                self.checksum,
                self.total_bytes,
                self.chunk_size,
            )
            .unwrap()
    }

    pub fn parse(metadata_bytes: &Vec<u8>) -> Result<Self, String> {
        let version_structure = structure!("B");
        let version_metadata =
            match version_structure.unpack(&metadata_bytes[..version_structure.size()]) {
                Ok(version_metadata) => version_metadata,
                Err(error) => return Err(error.to_string()),
            };
        match version_metadata.0 {
            1 => {
                let metadata_structure = structure!("BBQQQ");
                let metadata =
                    match metadata_structure.unpack(&metadata_bytes[..metadata_structure.size()]) {
                        Ok(metadata) => metadata,
                        Err(error) => return Err(error.to_string()),
                    };
                Ok(FileMetadata {
                    format_version: metadata.0,
                    hashing_algorithm: metadata.1,
                    checksum: metadata.2,
                    total_bytes: metadata.3,
                    chunk_size: metadata.4,
                })
            }
            _ => Ok(FileMetadata {
                format_version: 0,
                hashing_algorithm: 0,
                checksum: 0,
                total_bytes: 0,
                chunk_size: 0,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum HashingAlgorithm {
    None = 0,
    Xxh3 = 1,
}
impl HashingAlgorithm {
    pub fn from_u8(value: u8) -> HashingAlgorithm {
        match value {
            0 => HashingAlgorithm::None,
            1 => HashingAlgorithm::Xxh3,
            _ => HashingAlgorithm::None,
        }
    }
}
impl fmt::Display for HashingAlgorithm {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{:?}", self)
    }
}

pub trait Hasher {
    fn update(&mut self, _: &[u8]);
    fn digest(&mut self) -> u64;
    fn reset(&mut self);
}

#[derive(Clone)]
pub struct DummyHasher {}
impl Hasher for DummyHasher {
    fn update(&mut self, _: &[u8]) {}
    fn digest(&mut self) -> u64 {
        0
    }
    fn reset(&mut self) {}
}

#[derive(Clone)]
pub struct Xxh3Hasher {
    pub hasher: Xxh3,
}
impl Hasher for Xxh3Hasher {
    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn digest(&mut self) -> u64 {
        self.hasher.digest()
    }

    fn reset(&mut self) {
        self.hasher.reset()
    }
}

pub struct Encrypter<'a> {
    pub hasher: Box<dyn Hasher + 'a>,
    pub total_bytes: u64,
}

impl<'a> Encrypter<'a> {
    pub fn new(hashing_algorithm: &HashingAlgorithm) -> Encrypter<'a> {
        let hasher: Box<dyn Hasher> = match hashing_algorithm {
            HashingAlgorithm::None => Box::new(DummyHasher {}),
            HashingAlgorithm::Xxh3 => Box::new(Xxh3Hasher {
                hasher: Xxh3::new(),
            }),
        };
        Encrypter {
            hasher,
            total_bytes: 0,
        }
    }

    pub fn get_checksum(&mut self) -> u64 {
        self.hasher.digest()
    }

    pub fn encrypt(&mut self, fernet: &fernet::Fernet, data: &[u8]) -> String {
        self.total_bytes += data.len() as u64;
        self.hasher.update(&data);
        fernet.encrypt(&data)
    }
}
