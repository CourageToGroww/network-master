use sha2::{Digest, Sha256};

/// Generate a random API key with the given prefix.
pub fn generate_api_key() -> String {
    let random_bytes: [u8; 32] = rand_bytes();
    let hex = hex_encode(&random_bytes);
    format!("nm_ak_{hex}")
}

/// Compute SHA-256 hex digest of a byte slice.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex_encode(&hasher.finalize())
}

/// Compute a route hash from an ordered sequence of hop IPs.
pub fn route_hash(hop_ips: &[Option<String>]) -> String {
    let repr = format!("{:?}", hop_ips);
    sha256_hex(repr.as_bytes())
}

fn rand_bytes() -> [u8; 32] {
    let mut buf = [0u8; 32];
    let seed = format!(
        "{}-{:?}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::thread::current().id(),
        uuid::Uuid::new_v4()
    );
    let hash = Sha256::digest(seed.as_bytes());
    buf.copy_from_slice(&hash);
    buf
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_has_prefix() {
        let key = generate_api_key();
        assert!(key.starts_with("nm_ak_"));
        assert_eq!(key.len(), 6 + 64); // prefix + 32 bytes hex
    }

    #[test]
    fn sha256_deterministic() {
        let h1 = sha256_hex(b"hello");
        let h2 = sha256_hex(b"hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn route_hash_changes_on_different_routes() {
        let r1 = route_hash(&[Some("1.1.1.1".into()), Some("2.2.2.2".into())]);
        let r2 = route_hash(&[Some("1.1.1.1".into()), Some("3.3.3.3".into())]);
        assert_ne!(r1, r2);
    }
}
