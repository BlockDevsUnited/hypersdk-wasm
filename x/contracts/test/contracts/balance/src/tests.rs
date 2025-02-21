#[cfg(test)]
mod tests {
    use super::*;
    use wasmlanche_test::create_test_context;

    #[tokio::test]
    async fn test_get_balance() {
        let mut context = create_test_context().await;
        let addr = context.actor();
        
        // Initial balance should be 0
        let balance = get_balance(&mut context).await;
        assert_eq!(balance, 0);

        // Set some balance
        context.set_balance(&addr, 100).await.unwrap();
        
        // Check if balance is updated
        let balance = get_balance(&mut context).await;
        assert_eq!(balance, 100);
    }

    #[tokio::test]
    async fn test_set_balance() {
        let mut context = create_test_context().await;
        let addr = context.actor();
        
        // Set initial balance
        context.set_balance(&addr, 200).await.unwrap();
        
        // Transfer some amount
        set_balance(&mut context, 50).await;
        
        // Balance should remain 200 since we transferred to ourselves
        let balance = context.get_balance(&addr).await.unwrap();
        assert_eq!(balance, 200);
    }
}
