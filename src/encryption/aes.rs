use std::{sync::Arc, time::SystemTime};

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
        // Ok(Nonce::assume_unique_for_key(buffer))
        Ok(Nonce::assume_unique_for_key([0; 12]))
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
    sealing_key: SealingKey<AesNonceSequence>,
    opening_key: OpeningKey<AesNonceSequence>,
}

impl Aes {
    pub fn new(key: &[u8]) -> Result<Self, EncryptionError> {
        let unbound_sealing_key = UnboundKey::new(&AES_256_GCM, key)?;
        let unbound_opening_key = UnboundKey::new(&AES_256_GCM, key)?;
        let generator = Arc::new(AesNonceGenerator::new());
        let sealing_key = SealingKey::new(
            unbound_sealing_key,
            AesNonceSequence::new(generator.clone()),
        );
        let opening_key = OpeningKey::new(unbound_opening_key, AesNonceSequence::new(generator));
        Ok(Self {
            sealing_key,
            opening_key,
        })
    }

    pub fn encrypt(&mut self, data: &mut Vec<u8>) -> Result<(), EncryptionError> {
        Ok(self
            .sealing_key
            .seal_in_place_append_tag(Aad::empty(), data)?)
    }
    pub fn decrypt(&mut self, data: &mut Vec<u8>) -> Result<(), EncryptionError> {
        Ok(self
            .opening_key
            .open_in_place(Aad::empty(), data)
            .map(|_| ())?)
    }
}

#[cfg(test)]
mod test {
    use base64::Engine;

    use super::Aes;

    type Res = Result<(), Box<dyn std::error::Error>>;
    #[test]
    fn test_encrypt_decrypt() -> Res {
        let key_string = "bs6UDssSfq/jN2U5crEhmjWkUmIDfm4BfCXJ1uPKN+k=";
        let engine = base64::engine::general_purpose::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::general_purpose::GeneralPurposeConfig::new(),
        );
        let key_bytes = engine.decode(key_string)?;
        println!("{}", key_bytes.len());
        let mut aes = Aes::new(&key_bytes)?;
        println!("2");
        let message = "hello";
        let mut message_bytes = message.as_bytes().to_vec();
        aes.encrypt(&mut message_bytes)?;
        println!("{}", message_bytes.len());
        aes.decrypt(&mut message_bytes)?;
        println!("4");
        assert_eq!(std::str::from_utf8(&message_bytes)?, message);
        Ok(())
    }
}
