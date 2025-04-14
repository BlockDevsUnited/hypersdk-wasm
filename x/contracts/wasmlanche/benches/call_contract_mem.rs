// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::hint::black_box;
use wasmlanche::types::WasmlAddress;
use wasmlanche_test::Builder;

mod contracts;

iai::main!(
    call_contract,
    call_contract_nft,
    read_state,
    write_state,
    delete_state,
    get_balance,
    set_balance,
    emit_event,
    get_events,
    store_schema,
    get_schema,
    parallel_reads,
    parallel_events
);

fn call_contract() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    black_box(contract.always_true());
}

fn call_contract_nft() {
    let mut nft = contracts::Nft::new(Builder::new("test-crate"));
    let address = WasmlAddress::new([1; 32]);
    black_box(nft.mint(address, 1));
}

fn read_state() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    let key = b"test_key".to_vec();
    black_box(contract.read_state(&key));
}

fn write_state() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    let key = b"test_key".to_vec();
    let value = b"test_value".to_vec();
    black_box(contract.write_state(&key, &value));
}

fn delete_state() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    let key = b"test_key".to_vec();
    black_box(contract.delete_state(&key));
}

fn get_balance() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    let address = WasmlAddress::new([1; 32]);
    black_box(contract.get_balance(&address));
}

fn set_balance() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    let address = WasmlAddress::new([1; 32]);
    black_box(contract.set_balance(&address, 1000));
}

fn emit_event() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    let event = b"test_event".to_vec();
    black_box(contract.emit_event(&event));
}

fn get_events() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    black_box(contract.get_events());
}

fn store_schema() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    let schema = b"test_schema".to_vec();
    black_box(contract.store_schema(&schema));
}

fn get_schema() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    black_box(contract.get_schema());
}

fn parallel_reads() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    for i in 0..100 {
        let key = format!("key_{}", i).into_bytes();
        black_box(contract.read_state(&key));
    }
}

fn parallel_events() {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    for i in 0..100 {
        let event = format!("event_{}", i).into_bytes();
        black_box(contract.emit_event(&event));
    }
}
