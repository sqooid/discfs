use std::{sync::Arc, time::SystemTime};

use base64::Engine;
use log::{debug, trace};
use ring::{
    aead::*,
    rand::{SecureRandom, SystemRandom},
};

use crate::error::encryption::EncryptionError;

pub struct AesNonceGenerator {
    random: SystemRandom,
}

impl AesNonceGenerator {
    pub fn new() -> Self {
        let random = SystemRandom::new();
        // Force initialisation
        let _ = random.fill(&mut []);
        Self { random }
    }

    fn generate_nonce(&self) -> Result<Nonce, ring::error::Unspecified> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64);
        let mut buffer: [u8; 12] = [0; 12];
        if let Ok(t) = timestamp {
            buffer[..8].clone_from_slice(&t.to_be_bytes());
            self.random.fill(&mut buffer[8..])?;
        } else {
            self.random.fill(&mut buffer)?;
        }
        Ok(Nonce::assume_unique_for_key(buffer))
    }
}

pub struct AesNonceSequence {
    aes: Arc<AesNonceGenerator>,
}

impl AesNonceSequence {
    pub fn new(generator: Arc<AesNonceGenerator>) -> Self {
        Self { aes: generator }
    }
}

impl NonceSequence for AesNonceSequence {
    fn advance(&mut self) -> Result<Nonce, ring::error::Unspecified> {
        self.aes.generate_nonce()
    }
}

pub struct Aes {
    key: LessSafeKey,
    generator: AesNonceGenerator,
}

impl Aes {
    pub fn new(key: &[u8]) -> Result<Self, EncryptionError> {
        let unbound_key = UnboundKey::new(&AES_256_GCM, key)?;
        let generator = AesNonceGenerator::new();
        let key = LessSafeKey::new(unbound_key);
        Ok(Self { key, generator })
    }

    pub fn from_env(env_key: &str) -> Result<Self, EncryptionError> {
        let key_string = std::env::var(env_key)?;
        let engine = base64::engine::general_purpose::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::general_purpose::GeneralPurposeConfig::new(),
        );
        let key_bytes = engine.decode(key_string)?;
        Self::new(&key_bytes)
    }

    pub fn encrypt<'a>(&self, data: &'a mut Vec<u8>) -> Result<&'a [u8], EncryptionError> {
        let nonce = self.generator.generate_nonce()?;
        let nonce_bytes = nonce.as_ref().to_owned();
        trace!("nonce: {:x?}", nonce_bytes);
        let original_len = data.len();
        self.key
            .seal_in_place_append_tag(nonce, Aad::empty(), data)?;
        data.extend_from_slice(&nonce_bytes);
        debug!("encrypted {} bytes to {} bytes", original_len, data.len());
        Ok(&data[..])
    }

    pub fn decrypt<'a>(&self, data: &'a mut Vec<u8>) -> Result<&'a [u8], EncryptionError> {
        let original_len = data.len();
        let mut nonce_bytes: [u8; NONCE_LEN] = [0; NONCE_LEN];
        nonce_bytes[..].clone_from_slice(&data[data.len() - NONCE_LEN..]);
        data.truncate(data.len() - NONCE_LEN);
        trace!("nonce: {:x?}", nonce_bytes);
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        let result = self.key.open_in_place(nonce, Aad::empty(), data)?;
        debug!("decrypted {} bytes to {} bytes", original_len, result.len());
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use base64::Engine;
    use ring::aead::AES_256_GCM;

    use super::Aes;

    type Res = Result<(), Box<dyn std::error::Error>>;
    #[test]
    fn test_encrypt_decrypt() -> Res {
        println!("tag length: {}", AES_256_GCM.tag_len());
        println!("nonce length: {}", AES_256_GCM.nonce_len());
        let key_string = "bs6UDssSfq/jN2U5crEhmjWkUmIDfm4BfCXJ1uPKN+k=";
        let engine = base64::engine::general_purpose::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::general_purpose::GeneralPurposeConfig::new(),
        );
        let key_bytes = engine.decode(key_string)?;
        println!("key length: {}", key_bytes.len());
        let mut aes = Aes::new(&key_bytes)?;
        let message = "hello";
        let mut message_bytes = message.as_bytes().to_vec();
        println!("message: {:x?}", message_bytes);
        aes.encrypt(&mut message_bytes)?;
        println!("encrypted: {} bytes", message_bytes.len());
        let decrypted = aes.decrypt(&mut message_bytes)?;
        println!("message: {:x?}", decrypted);
        assert_eq!(std::str::from_utf8(&decrypted)?, message);
        Ok(())
    }
}
