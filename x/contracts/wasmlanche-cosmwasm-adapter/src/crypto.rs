use cosmwasm_std::StdError;
use k256::ecdsa::{SigningKey, VerifyingKey, Signature};
use k256::elliptic_curve::generic_array::GenericArray;
use k256::elliptic_curve::generic_array::typenum::U32;
use ed25519_dalek::{Signer, Verifier};
use ed25519_dalek::{SigningKey as Ed25519SigningKey, VerifyingKey as Ed25519VerifyingKey};
use rand::rngs::OsRng;

#[derive(Debug)]
pub enum CryptoError {
    InvalidKey,
    InvalidSignature,
    InternalError(String),
}

#[derive(Default)]
pub struct CryptoApi;

impl CryptoApi {
    pub fn secp256k1_sign(&self, message: &[u8], secret_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if secret_key.len() != 32 {
            return Err(CryptoError::InvalidKey);
        }
        
        let key_array = GenericArray::clone_from_slice(secret_key);
        let signing_key = SigningKey::from_bytes(&key_array)
            .map_err(|_| CryptoError::InvalidKey)?;
        
        let signature: Signature = signing_key.sign(message);
        Ok(signature.to_vec())
    }

    pub fn secp256k1_verify(&self, message: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool, CryptoError> {
        if signature.len() != 64 {
            return Err(CryptoError::InvalidSignature);
        }
        
        if public_key.len() != 33 && public_key.len() != 65 {
            return Err(CryptoError::InvalidKey);
        }
        
        let verifying_key = VerifyingKey::from_sec1_bytes(public_key)
            .map_err(|_| CryptoError::InvalidKey)?;
        
        let signature = Signature::from_slice(signature)
            .map_err(|_| CryptoError::InvalidSignature)?;
        
        Ok(verifying_key.verify(message, &signature).is_ok())
    }

    pub fn ed25519_generate_key(&self) -> Result<(Vec<u8>, Vec<u8>), StdError> {
        let mut csprng = OsRng;
        let signing_key = Ed25519SigningKey::generate(&mut csprng);
        let verifying_key = Ed25519VerifyingKey::from(&signing_key);
        
        Ok((signing_key.to_bytes().to_vec(), verifying_key.to_bytes().to_vec()))
    }

    pub fn ed25519_sign(&self, message: &[u8], secret_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let secret_key_bytes: [u8; 32] = secret_key.try_into()
            .map_err(|_| CryptoError::InvalidKey)?;
            
        let signing_key = Ed25519SigningKey::from_bytes(&secret_key_bytes);
        let signature = signing_key.sign(message);
        Ok(signature.to_vec())
    }

    pub fn ed25519_verify(&self, message: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool, CryptoError> {
        let public_key_bytes: [u8; 32] = public_key.try_into()
            .map_err(|_| CryptoError::InvalidKey)?;
            
        let verifying_key = Ed25519VerifyingKey::from_bytes(&public_key_bytes)
            .map_err(|_| CryptoError::InvalidKey)?;

        let signature_bytes: [u8; 64] = signature.try_into()
            .map_err(|_| CryptoError::InvalidSignature)?;

        Ok(verifying_key.verify(message, &ed25519_dalek::Signature::from_bytes(&signature_bytes)).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secp256k1_success() {
        let api = CryptoApi::default();
        let msg = b"test message";
        let privkey = GenericArray::<u8, U32>::from_slice(&[1u8; 32]);
        
        // Test signing
        let sig = api.secp256k1_sign(msg, privkey.as_slice()).unwrap();
        
        // Get public key
        let signing_key = SigningKey::from_bytes(privkey).unwrap();
        let pubkey = signing_key.verifying_key().to_encoded_point(false).to_bytes();
        
        // Test verification
        assert!(api.secp256k1_verify(msg, &sig, &pubkey).unwrap());
    }

    #[test]
    fn test_secp256k1_invalid_inputs() {
        let api = CryptoApi::default();
        let msg = b"test message";
        
        // Test invalid private key
        let invalid_privkey = vec![1u8; 31]; // Wrong length
        assert!(api.secp256k1_sign(msg, &invalid_privkey).is_err());
        
        // Test invalid signature
        let invalid_sig = vec![0u8; 63]; // Invalid signature bytes
        let valid_pubkey = vec![1u8; 65]; // Valid length public key
        assert!(api.secp256k1_verify(msg, &invalid_sig, &valid_pubkey).is_err());
        
        // Test invalid public key
        let valid_sig = vec![1u8; 64]; // Valid length signature
        let invalid_pubkey = vec![1u8; 64]; // Wrong length
        assert!(api.secp256k1_verify(msg, &valid_sig, &invalid_pubkey).is_err());
    }

    #[test]
    fn test_ed25519_success() {
        let api = CryptoApi::default();
        let msg = b"test message";
        
        // Generate key pair
        let (privkey, pubkey) = api.ed25519_generate_key().unwrap();
        
        // Test signing
        let sig = api.ed25519_sign(msg, &privkey).unwrap();
        
        // Test verification
        assert!(api.ed25519_verify(msg, &sig, &pubkey).unwrap());
        
        // Test with different message
        let different_msg = b"different message";
        assert!(!api.ed25519_verify(different_msg, &sig, &pubkey).unwrap());
    }

    #[test]
    fn test_ed25519_invalid_inputs() {
        let api = CryptoApi::default();
        let msg = b"test message";
        
        // Test invalid private key
        let invalid_privkey = vec![1u8; 31]; // Wrong length
        assert!(api.ed25519_sign(msg, &invalid_privkey).is_err());
        
        // Generate valid keypair for testing
        let (_, pubkey) = api.ed25519_generate_key().unwrap();
        
        // Test invalid signature
        let invalid_sig = vec![0u8; 63]; // Wrong length
        assert!(api.ed25519_verify(msg, &invalid_sig, &pubkey).is_err());
        
        // Test invalid public key
        let valid_sig = vec![1u8; 64]; // Valid length signature
        let invalid_pubkey = vec![1u8; 31]; // Wrong length
        assert!(api.ed25519_verify(msg, &valid_sig, &invalid_pubkey).is_err());
    }
}
