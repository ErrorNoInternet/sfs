use std::fmt;
use xxhash_rust::xxh3::Xxh3;
use xxhash_rust::xxh32::Xxh32;
use xxhash_rust::xxh64::Xxh64;
#[macro_use]
extern crate structure;

pub const SFS_FORMAT_VERSION: u8 = 2;
pub const SFS_VERSION_STRING: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Default, Clone)]
pub struct FileMetadata {
    pub format_version: u8,
    pub original_name: String,
    pub restore_name: bool,
    pub total_bytes: u64,
    pub hashing_algorithm: u8,
    pub checksum: u64,
    pub chunk_size: u64,
}
impl FileMetadata {
    pub fn pack(&self) -> Vec<u8> {
        let mut original_name = self.original_name.clone().into_bytes().to_vec();
        for _ in 0..255 - original_name.len() {
            original_name.push(0)
        }

        let metadata_structure = structure!("B255S?QBQQ");
        metadata_structure
            .pack(
                self.format_version,
                &original_name,
                self.restore_name,
                self.total_bytes,
                self.hashing_algorithm,
                self.checksum,
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
                    original_name: String::from("UNSUPPORTED"),
                    restore_name: false,
                })
            }
            2 => {
                let metadata_structure = structure!("B255S?QBQQ");
                let metadata =
                    match metadata_structure.unpack(&metadata_bytes[..metadata_structure.size()]) {
                        Ok(metadata) => metadata,
                        Err(error) => return Err(error.to_string()),
                    };
                Ok(FileMetadata {
                    format_version: metadata.0,
                    original_name: std::str::from_utf8(&metadata.1)
                        .unwrap_or_default()
                        .trim_matches(char::from(0))
                        .to_string(),
                    restore_name: metadata.2,
                    total_bytes: metadata.3,
                    hashing_algorithm: metadata.4,
                    checksum: metadata.5,
                    chunk_size: metadata.6,
                })
            }
            _ => Ok(FileMetadata {
                format_version: version_metadata.0,
                original_name: String::new(),
                restore_name: false,
                total_bytes: 0,
                hashing_algorithm: 0,
                checksum: 0,
                chunk_size: 0,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HashingAlgorithm {
    None = 0,
    Xxh3 = 1,
    Xxh64 = 2,
    Xxh32 = 3,
}
impl fmt::Display for HashingAlgorithm {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", format!("{:?}", self).to_uppercase())
    }
}
impl HashingAlgorithm {
    pub fn list() -> &'static [HashingAlgorithm] {
        &[
            HashingAlgorithm::None,
            HashingAlgorithm::Xxh3,
            HashingAlgorithm::Xxh64,
            HashingAlgorithm::Xxh32,
        ]
    }

    pub fn from_u8(value: u8) -> HashingAlgorithm {
        let mut value = value.into();
        let hashing_algorithm_list = HashingAlgorithm::list();
        if value >= hashing_algorithm_list.len() {
            value = 0
        }
        hashing_algorithm_list[value]
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

pub struct Xxh64Hasher {
    pub hasher: Xxh64,
}
impl Hasher for Xxh64Hasher {
    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn digest(&mut self) -> u64 {
        self.hasher.digest()
    }

    fn reset(&mut self) {
        self.hasher.reset(0)
    }
}

pub struct Xxh32Hasher {
    pub hasher: Xxh32,
}
impl Hasher for Xxh32Hasher {
    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn digest(&mut self) -> u64 {
        self.hasher.digest().into()
    }

    fn reset(&mut self) {
        self.hasher.reset(0)
    }
}

fn get_hasher(hashing_algorithm: HashingAlgorithm) -> Box<dyn Hasher> {
    match hashing_algorithm {
        HashingAlgorithm::None => Box::new(DummyHasher {}),
        HashingAlgorithm::Xxh3 => Box::new(Xxh3Hasher {
            hasher: Xxh3::new(),
        }),
        HashingAlgorithm::Xxh64 => Box::new(Xxh64Hasher {
            hasher: Xxh64::new(0),
        }),
        HashingAlgorithm::Xxh32 => Box::new(Xxh32Hasher {
            hasher: Xxh32::new(0),
        }),
    }
}

pub struct Encrypter<'a> {
    pub fernet: fernet::Fernet,
    pub hasher: Box<dyn Hasher + 'a>,
    pub total_bytes: u64,
}
impl<'a> Encrypter<'a> {
    pub fn new(fernet: fernet::Fernet, hashing_algorithm: HashingAlgorithm) -> Encrypter<'a> {
        Encrypter {
            fernet,
            hasher: get_hasher(hashing_algorithm),
            total_bytes: 0,
        }
    }

    pub fn get_checksum(&mut self) -> u64 {
        self.hasher.digest()
    }

    pub fn encrypt(&mut self, data: &[u8]) -> String {
        self.total_bytes += data.len() as u64;
        self.hasher.update(&data);
        self.fernet.encrypt(&data)
    }
}

pub struct Decrypter<'a> {
    pub fernet: fernet::Fernet,
    pub hasher: Box<dyn Hasher + 'a>,
    pub total_bytes: u64,
}
impl<'a> Decrypter<'a> {
    pub fn new(fernet: fernet::Fernet, hashing_algorithm: HashingAlgorithm) -> Decrypter<'a> {
        Decrypter {
            fernet,
            hasher: get_hasher(hashing_algorithm),
            total_bytes: 0,
        }
    }

    pub fn get_checksum(&mut self) -> u64 {
        self.hasher.digest()
    }

    pub fn decrypt(&mut self, encrypted_data: &str) -> Result<Vec<u8>, String> {
        match self.fernet.decrypt(&encrypted_data) {
            Ok(data) => {
                self.total_bytes += data.len() as u64;
                self.hasher.update(&data);
                Ok(data)
            }
            Err(error) => Err(error.to_string()),
        }
    }
}
