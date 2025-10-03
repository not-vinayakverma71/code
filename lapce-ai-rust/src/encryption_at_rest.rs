/// Encryption at Rest - Day 46 AM
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce
};
use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier};
use rand::{RngCore, thread_rng};
use anyhow::Result;

pub struct EncryptionManager {
    master_key: Vec<u8>,
    key_derivation: KeyDerivation,
}

pub struct KeyDerivation {
    salt: [u8; 32],
    iterations: u32,
}

impl EncryptionManager {
    pub fn new(password: &str) -> Result<Self> {
        let mut salt = [0u8; 32];
        thread_rng().fill_bytes(&mut salt);
        
        let master_key = Self::derive_key(password, &salt)?;
        
        Ok(Self {
            master_key,
            key_derivation: KeyDerivation {
                salt,
                iterations: 100_000,
            },
        })
    }
    
    fn derive_key(password: &str, salt: &[u8]) -> Result<Vec<u8>> {
        let mut key = vec![0u8; 32];
        pbkdf2::pbkdf2_hmac::<sha2::Sha256>(
            password.as_bytes(),
            salt,
            100_000,
            &mut key
        );
        Ok(key)
    }
    
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let key = Key::<Aes256Gcm>::from_slice(&self.master_key);
        let cipher = Aes256Gcm::new(key);
        
        let mut nonce_bytes = [0u8; 12];
        thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
    
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 12 {
            return Err(anyhow::anyhow!("Invalid ciphertext"));
        }
        
        let (nonce_bytes, encrypted) = ciphertext.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let key = Key::<Aes256Gcm>::from_slice(&self.master_key);
        let cipher = Aes256Gcm::new(key);
        
        let plaintext = cipher.decrypt(nonce, encrypted)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
        
        Ok(plaintext)
    }
    
    pub fn encrypt_file(&self, input_path: &str, output_path: &str) -> Result<()> {
        let plaintext = std::fs::read(input_path)?;
        let ciphertext = self.encrypt(&plaintext)?;
        std::fs::write(output_path, ciphertext)?;
        Ok(())
    }
    
    pub fn decrypt_file(&self, input_path: &str, output_path: &str) -> Result<()> {
        let ciphertext = std::fs::read(input_path)?;
        let plaintext = self.decrypt(&ciphertext)?;
        std::fs::write(output_path, plaintext)?;
        Ok(())
    }
}
