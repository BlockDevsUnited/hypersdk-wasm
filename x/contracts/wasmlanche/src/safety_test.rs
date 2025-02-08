#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_depth() {
        let mut context = SafetyContext::new();
        
        // Test successful calls within limit
        for _ in 0..MAX_CALL_DEPTH {
            assert!(context.enter_call().is_ok());
        }

        // Test exceeding max depth
        assert!(context.enter_call().is_err());

        // Test call exit
        context.exit_call();
        assert!(context.enter_call().is_ok());
    }

    #[test]
    fn test_nonce_verification() {
        let mut context = SafetyContext::new();
        let actor = vec![1, 2, 3];

        // Test initial nonce
        assert_eq!(context.get_nonce(&actor), 0);

        // Test successful nonce verification and increment
        assert!(context.verify_and_increment_nonce(&actor, 0).is_ok());
        assert_eq!(context.get_nonce(&actor), 1);

        // Test invalid nonce
        assert!(context.verify_and_increment_nonce(&actor, 0).is_err());
        assert!(context.verify_and_increment_nonce(&actor, 2).is_err());
        assert_eq!(context.get_nonce(&actor), 1);

        // Test successful sequential nonces
        assert!(context.verify_and_increment_nonce(&actor, 1).is_ok());
        assert!(context.verify_and_increment_nonce(&actor, 2).is_ok());
        assert_eq!(context.get_nonce(&actor), 3);
    }

    #[test]
    fn test_protocol_version() {
        let context = SafetyContext::new();

        // Test matching version
        assert!(context.check_protocol_version(PROTOCOL_VERSION).is_ok());

        // Test mismatched version
        assert!(context.check_protocol_version(PROTOCOL_VERSION + 1).is_err());
        assert!(context.check_protocol_version(PROTOCOL_VERSION - 1).is_err());
    }

    #[test]
    fn test_safety_manager() {
        let manager = SafetyManager::new();
        let actor = vec![1, 2, 3];

        // Test thread-safe operations
        assert!(manager.enter_call().is_ok());
        assert!(manager.verify_and_increment_nonce(&actor, 0).is_ok());
        assert_eq!(manager.get_nonce(&actor), 1);
        assert!(manager.check_protocol_version(PROTOCOL_VERSION).is_ok());
        manager.exit_call();
    }
}
