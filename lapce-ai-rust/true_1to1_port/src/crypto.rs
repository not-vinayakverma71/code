// EXACT translation of crypto.randomBytes from Node.js
use rand::Rng;

// Line-by-line match of: crypto.randomBytes(6).toString("hex")
pub fn random_bytes(size: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    (0..size).map(|_| rng.gen()).collect()
}

pub fn to_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}
