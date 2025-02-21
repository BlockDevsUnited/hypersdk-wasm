// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use iai::black_box;
use wasmlanche::types::Address;
use wasmlanche_test::create_test_context;

fn bench_call_contract() -> Vec<u8> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut context = create_test_context();
    let target = Address::from([0u8; 33]);

    rt.block_on(async {
        context
            .call_contract(&target.as_bytes(), "test", &[], black_box(1000))
            .await
            .unwrap()
    })
}

iai::main!(bench_call_contract);
