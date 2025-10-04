/// TLS Configuration - Day 46 PM
use rustls::{Certificate, PrivateKey, ServerConfig, ClientConfig};
use rustls_pemfile;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use anyhow::Result;

pub struct TLSConfig {
    pub server_config: Arc<ServerConfig>,
    pub client_config: Arc<ClientConfig>,
    pub cipher_suites: Vec<String>,
    pub min_tls_version: TlsVersion,
}

#[derive(Debug, Clone)]
pub enum TlsVersion {
    TLS12,
    TLS13,
}

impl TLSConfig {
    pub fn new(cert_path: &str, key_path: &str) -> Result<Self> {
        let certs = Self::load_certs(cert_path)?;
        let key = Self::load_private_key(key_path)?;
        
        let server_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs.clone(), key)?;
        
        let client_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        
        Ok(Self {
            server_config: Arc::new(server_config),
            client_config: Arc::new(client_config),
            cipher_suites: vec![
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_AES_128_GCM_SHA256".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
            ],
            min_tls_version: TlsVersion::TLS13,
        })
    }
    
    fn load_certs(path: &str) -> Result<Vec<Certificate>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let certs = rustls_pemfile::certs(&mut reader)?
            .into_iter()
            .map(Certificate)
            .collect();
        Ok(certs)
    }
    
    fn load_private_key(path: &str) -> Result<PrivateKey> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)?;
        
        if keys.is_empty() {
            return Err(anyhow::anyhow!("No private key found"));
        }
        
        Ok(PrivateKey(keys[0].clone()))
    }
    
    pub fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>)> {
        // Generate self-signed certificate for testing
        let cert = b"-----BEGIN CERTIFICATE-----
MIIBkTCB+wIJAKHHIG...
-----END CERTIFICATE-----";
        
        let key = b"-----BEGIN PRIVATE KEY-----
MIICdgIBADANBg...
-----END PRIVATE KEY-----";
        
        Ok((cert.to_vec(), key.to_vec()))
    }
}
