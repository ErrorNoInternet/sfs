use xxhash_rust::xxh3::Xxh3;

pub const SFS_VERSION: u8 = 1;

pub struct Encrypter {
    pub hasher: Xxh3,
    pub total_bytes: u64,
}

impl Encrypter {
    pub fn new() -> Self {
        Self {
            hasher: Xxh3::new(),
            total_bytes: 0,
        }
    }

    pub fn get_checksum(&mut self) -> (bool, u64) {
        let hash = self.hasher.digest();
        self.hasher.reset();
        if self.hasher.digest() == hash {
            (false, 0)
        } else {
            (true, hash)
        }
    }

    pub fn encrypt_with_hash(&mut self, fernet: &fernet::Fernet, data: &[u8]) -> String {
        self.total_bytes += data.len() as u64;
        self.hasher.update(&data);
        fernet.encrypt(&data)
    }

    pub fn encrypt(&mut self, fernet: &fernet::Fernet, data: &[u8]) -> String {
        self.total_bytes += data.len() as u64;
        fernet.encrypt(&data)
    }
}
