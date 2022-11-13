use std::fmt;
use xxhash_rust::xxh3::Xxh3;

pub const CURRENT_SFS_VERSION: u8 = 1;

pub fn resolve_version_string(version: u8) -> &'static str {
    match version {
        1 => "1.0.0",
        _ => "?",
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
