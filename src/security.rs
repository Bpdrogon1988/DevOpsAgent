use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::env;
use hex;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_github_signature(payload: &str, signature: &str) -> bool {
    let secret = env::var("GITHUB_WEBHOOK_SECRET").unwrap_or_else(|_| "default_secret".into());
    
    // GitHub sends signature as: sha256=HEX_STRING
    let hex_sig = if signature.starts_with("sha256=") {
        &signature[7..]
    } else {
        return false;
    };

    let decoded_sig: Vec<u8> = match hex::decode(hex_sig) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
        
    mac.update(payload.as_bytes());
    
    mac.verify_slice(&decoded_sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_signature() {
        unsafe { env::set_var("GITHUB_WEBHOOK_SECRET", "my_secret"); }
        let payload = "{\"action\": \"opened\"}";
        
        let mut mac = HmacSha256::new_from_slice(b"my_secret").unwrap();
        mac.update(payload.as_bytes());
        let expected_signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        assert!(verify_github_signature(payload, &expected_signature));
    }

    #[test]
    fn test_invalid_signature() {
        unsafe { env::set_var("GITHUB_WEBHOOK_SECRET", "my_secret"); }
        let payload = "{\"action\": \"opened\"}";
        let bad_signature = "sha256=deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

        assert!(!verify_github_signature(payload, bad_signature));
    }

    #[test]
    fn test_missing_sha256_prefix() {
        unsafe { env::set_var("GITHUB_WEBHOOK_SECRET", "my_secret"); }
        let payload = "{\"action\": \"opened\"}";
        let bad_signature = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

        assert!(!verify_github_signature(payload, bad_signature));
    }
}
