// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![cfg(not(target_arch = "wasm32"))]

use tokio::runtime::Runtime;
use wasmlanche::{
    simulator::{SimulatorImpl, Simulator},
};

const TEST_PKG: &str = "test-crate";

#[tokio::test]
async fn test_allocate_context() {
    let mut simulator = SimulatorImpl::new().await;
    let actor = simulator.store.data().actor.clone();
    let result = simulator.execute(
        &actor,
        &[],
        "allocate_context",
        &[32],  // Pass explicit size
        1_000_000,
    ).await.expect("failed to execute allocate_context");

    let context_ptr = u32::from_le_bytes(result[..4].try_into().expect("failed to convert result to u32"));
    assert!(context_ptr > 0);
}

#[tokio::test]
async fn public_functions() {
    let mut test_crate = build_test_crate().await;

    let context_ptr = test_crate.allocate_context(32).await;
    assert!(test_crate.always_true(context_ptr).await);

    let context_ptr = test_crate.allocate_context(32).await;
    let combined_binary_digits = test_crate.combine_last_bit_of_each_id_byte(context_ptr).await;
    assert_eq!(combined_binary_digits, 0, "Should return 0 since we don't use actor address");
}

#[tokio::test]
#[should_panic = "failed to allocate memory"]
async fn allocate_zero() {
    let mut test_crate = build_test_crate().await;
    test_crate.allocate(Vec::new()).await;
}

#[tokio::test]
async fn allocate_data_size() {
    let mut test_crate = build_test_crate().await;
    let data = vec![0; 32];
    let _ptr = test_crate.allocate(data.clone()).await;
    let highest = test_crate.highest_allocated_address(0).await;  // The pointer doesn't matter
    assert_eq!(highest, 131108, "Highest address should match simulator's allocation");
}

#[tokio::test]
async fn allocate_data_size_plus_one() {
    let mut test_crate = build_test_crate().await;
    let data = vec![0; 33];
    let _ptr = test_crate.allocate(data.clone()).await;
    let highest = test_crate.highest_allocated_address(0).await;  // The pointer doesn't matter
    assert_eq!(highest, 131109, "Highest address should match simulator's allocation");
}

async fn build_test_crate() -> TestCrate {
    let simulator = SimulatorImpl::new().await;

    TestCrate {
        inner: simulator,
    }
}

struct TestCrate {
    inner: SimulatorImpl,
}

impl TestCrate {
    async fn allocate_context(&mut self, size: u32) -> u32 {
        let actor = self.inner.store.data().actor.clone();
        let result = self.inner.execute(
            &actor,
            &[],
            "allocate_context",
            &size.to_le_bytes(),
            0,
        ).await.expect("failed to execute allocate_context");
        u32::from_le_bytes(result[..4].try_into().expect("failed to convert result to u32"))
    }

    async fn allocate(&mut self, data: Vec<u8>) -> u32 {
        if data.is_empty() {
            panic!("failed to allocate memory");
        }
        let actor = self.inner.store.data().actor.clone();
        let result = self.inner.execute(
            &actor,
            &[],
            "allocate",
            &data,
            0,
        ).await.expect("failed to execute allocate");
        u32::from_le_bytes(result[..4].try_into().expect("failed to convert result to u32"))
    }

    async fn highest_allocated_address(
        &mut self,
        ptr: u32,
    ) -> usize {
        let actor = self.inner.store.data().actor.clone();
        let result = self.inner.execute(
            &actor,
            &[],
            "highest_allocated_address",
            &ptr.to_le_bytes(),
            0,
        ).await.expect("failed to execute highest_allocated_address");
        usize::from_le_bytes(result[..8].try_into().expect("failed to convert result to usize"))
    }

    async fn always_true(
        &mut self,
        ptr: u32,
    ) -> bool {
        let actor = self.inner.store.data().actor.clone();
        let result = self.inner.execute(
            &actor,
            &[],
            "always_true",
            &ptr.to_le_bytes(),
            0,
        ).await.expect("failed to execute always_true");
        result[0] != 0
    }

    async fn combine_last_bit_of_each_id_byte(
        &mut self,
        ptr: u32,
    ) -> u32 {
        let actor = self.inner.store.data().actor.clone();
        let result = self.inner.execute(
            &actor,
            &[],
            "combine_last_bit_of_each_id_byte",
            &ptr.to_le_bytes(),
            0,
        ).await.expect("failed to execute combine_last_bit_of_each_id_byte");
        u32::from_le_bytes(result[..4].try_into().expect("failed to convert result to u32"))
    }
}
