use k256::ecdsa::{Signature as Secp256k1Signature, VerifyingKey as Secp256k1VerifyingKey, signature::Verifier as Secp256k1Verifier};
use ed25519_dalek::{Signature as Ed25519Signature, VerifyingKey as Ed25519VerifyingKey};
use sha2::{Sha256, Digest};

pub fn secp256k1_verify(message: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    // Convert public key bytes to VerifyingKey
    let verifying_key = match Secp256k1VerifyingKey::from_sec1_bytes(public_key) {
        Ok(key) => key,
        Err(_) => return false,
    };

    // Convert signature bytes to signature
    let signature = match Secp256k1Signature::from_der(signature) {
        Ok(sig) => sig,
        Err(_) => return false,
    };

    // Verify signature
    verifying_key.verify(message, &signature).is_ok()
}

pub fn secp256k1_recover_pubkey(_message: &[u8], _signature: &[u8], _recovery_param: u8) -> Option<Vec<u8>> {
    // This is a placeholder - actual implementation would require recoverable signatures
    None
}

pub fn ed25519_verify(message: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    // Convert public key bytes to VerifyingKey
    let public_key: [u8; 32] = match public_key.try_into() {
        Ok(key) => key,
        Err(_) => return false,
    };
    
    let verifying_key = match Ed25519VerifyingKey::from_bytes(&public_key) {
        Ok(key) => key,
        Err(_) => return false,
    };

    // Convert signature bytes to Signature
    let signature = match Ed25519Signature::from_slice(signature) {
        Ok(sig) => sig,
        Err(_) => return false,
    };

    // Verify signature
    verifying_key.verify_strict(message, &signature).is_ok()
}

pub fn ed25519_batch_verify(messages: &[&[u8]], signatures: &[&[u8]], public_keys: &[&[u8]]) -> bool {
    if messages.len() != signatures.len() || messages.len() != public_keys.len() {
        return false;
    }

    for ((message, signature), public_key) in messages.iter().zip(signatures).zip(public_keys) {
        if !ed25519_verify(message, signature, public_key) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::SecretKey;
    use ed25519_dalek::{SigningKey, Signer};
    use rand_core::OsRng;

    #[test]
    fn test_secp256k1_verify() {
        // Generate a key pair
        let secret_key = SecretKey::random(&mut OsRng);
        let signing_key = k256::ecdsa::SigningKey::from(secret_key);
        let verifying_key = Secp256k1VerifyingKey::from(&signing_key);
        let public_key = verifying_key.to_sec1_bytes();

        // Sign a message
        let message = b"test message";
        let signature: Secp256k1Signature = signing_key.sign(message);

        // Verify signature
        assert!(secp256k1_verify(
            message,
            signature.to_der().as_bytes(),
            &public_key
        ));
    }

    #[test]
    fn test_ed25519_verify() {
        // Generate a key pair
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let public_key = verifying_key.to_bytes();

        // Sign a message
        let message = b"test message";
        let signature = signing_key.sign(message);

        // Verify signature
        assert!(ed25519_verify(message, signature.to_bytes().as_slice(), &public_key));
    }

    #[test]
    fn test_ed25519_batch_verify() {
        // Generate key pairs
        let signing_key1 = SigningKey::generate(&mut OsRng);
        let signing_key2 = SigningKey::generate(&mut OsRng);
        let verifying_key1 = signing_key1.verifying_key();
        let verifying_key2 = signing_key2.verifying_key();

        let message1 = b"test message 1";
        let message2 = b"test message 2";

        let sig1_bytes = signing_key1.sign(message1).to_bytes();
        let sig2_bytes = signing_key2.sign(message2).to_bytes();
        let key1_bytes = verifying_key1.to_bytes();
        let key2_bytes = verifying_key2.to_bytes();

        let messages = [message1.as_slice(), message2.as_slice()];
        let signatures = [sig1_bytes.as_slice(), sig2_bytes.as_slice()];
        let public_keys = [key1_bytes.as_slice(), key2_bytes.as_slice()];

        assert!(ed25519_batch_verify(&messages, &signatures, &public_keys));
    }
}
