use hex;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_github_signature(payload: &[u8], signature: &str, secret: &str) -> bool {
    let hex_sig = if signature.starts_with("sha256=") {
        &signature[7..]
    } else {
        return false;
    };

    let decoded_sig: Vec<u8> = match hex::decode(hex_sig) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("invalid secret length");
    mac.update(payload);

    mac.verify_slice(&decoded_sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_signature() {
        let secret = "my_secret";
        let payload = b"{\"action\": \"opened\"}";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let expected_signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        assert!(verify_github_signature(
            payload,
            &expected_signature,
            secret
        ));
    }

    #[test]
    fn test_invalid_signature() {
        let secret = "my_secret";
        let payload = b"{\"action\": \"opened\"}";
        let bad_signature =
            "sha256=deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

        assert!(!verify_github_signature(payload, bad_signature, secret));
    }

    #[test]
    fn test_missing_sha256_prefix() {
        let secret = "my_secret";
        let payload = b"{\"action\": \"opened\"}";
        let bad_signature = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

        assert!(!verify_github_signature(payload, bad_signature, secret));
    }
}
