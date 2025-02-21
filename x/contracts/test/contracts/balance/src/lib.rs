// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use wasmlanche::Context;
use wasmlanche::error::Error;

#[no_mangle]
pub async fn get_balance(context: &mut Context) -> u64 {
    let addr = context.actor();
    context.get_balance(addr).await.unwrap_or(0)
}

#[no_mangle]
pub async fn set_balance(context: &mut Context, amount: u64) -> Result<(), Error> {
    let addr = context.actor().clone();
    context.transfer(&addr, &addr, amount).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmlanche_test::create_test_context;

    #[test]
    fn test_get_balance() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut context = create_test_context();
            
            // Initial balance should be 0
            let balance = get_balance(&mut context).await;
            assert_eq!(balance, 0);
        });
    }

    #[test]
    fn test_set_balance() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut context = create_test_context();
            let addr = context.actor().clone();
            
            // Initial balance should be 0
            let balance = get_balance(&mut context).await;
            assert_eq!(balance, 0);
            
            // Set initial balance to 200 (this should fail since we don't have funds)
            let result = set_balance(&mut context, 200).await;
            assert!(result.is_err(), "Expected set_balance to fail due to insufficient funds");
            
            // Balance should still be 0
            let balance = get_balance(&mut context).await;
            assert_eq!(balance, 0);
        });
    }
}
