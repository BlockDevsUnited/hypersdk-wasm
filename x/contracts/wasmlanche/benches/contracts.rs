// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use wasmlanche::types::WasmlAddress;
use wasmlanche_test::{Builder, TestCrate, UserDefinedFn};
use borsh::{BorshDeserialize, BorshSerialize};

pub struct Contract {
    inner: TestCrate,
    always_true: UserDefinedFn,
    read_state: UserDefinedFn,
    write_state: UserDefinedFn,
    delete_state: UserDefinedFn,
    get_balance: UserDefinedFn,
    set_balance: UserDefinedFn,
    emit_event: UserDefinedFn,
    get_events: UserDefinedFn,
    store_schema: UserDefinedFn,
    get_schema: UserDefinedFn,
}

impl Contract {
    #[inline]
    pub fn new(builder: Builder) -> Self {
        let mut inner = builder.build();
        let always_true = inner.get_user_defined_typed_func("always_true");
        let read_state = inner.get_user_defined_typed_func("read_state");
        let write_state = inner.get_user_defined_typed_func("write_state");
        let delete_state = inner.get_user_defined_typed_func("delete_state");
        let get_balance = inner.get_user_defined_typed_func("get_balance");
        let set_balance = inner.get_user_defined_typed_func("set_balance");
        let emit_event = inner.get_user_defined_typed_func("emit_event");
        let get_events = inner.get_user_defined_typed_func("get_events");
        let store_schema = inner.get_user_defined_typed_func("store_schema");
        let get_schema = inner.get_user_defined_typed_func("get_schema");

        Self { 
            inner, 
            always_true,
            read_state,
            write_state,
            delete_state,
            get_balance,
            set_balance,
            emit_event,
            get_events,
            store_schema,
            get_schema,
        }
    }

    #[inline]
    pub fn always_true(&mut self) -> bool {
        let Self { always_true, inner, .. } = self;
        let ctx = inner.allocate_context();

        always_true
            .call(inner.store_mut(), ctx)
            .expect("failed to call `always_true` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("always_true should always return something");

        bool::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }

    #[inline]
    pub fn read_state(&mut self, key: &[u8]) -> Vec<u8> {
        let Self { read_state, inner, .. } = self;
        let mut buf = Vec::new();
        key.serialize(&mut buf).expect("failed to serialize key");
        let params = inner.allocate(buf);
        let ctx = inner.allocate_context();

        read_state
            .call(inner.store_mut(), ctx)
            .expect("failed to call `read_state` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("read_state should always return something");

        Vec::<u8>::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }

    #[inline]
    pub fn write_state(&mut self, key: &[u8], value: &[u8]) {
        let Self { write_state, inner, .. } = self;
        let mut buf = Vec::new();
        key.serialize(&mut buf).expect("failed to serialize key");
        value.serialize(&mut buf).expect("failed to serialize value");
        let params = inner.allocate(buf);
        let ctx = inner.allocate_context();

        write_state
            .call(inner.store_mut(), ctx)
            .expect("failed to call `write_state` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("write_state should always return something");

        <()>::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }

    #[inline]
    pub fn delete_state(&mut self, key: &[u8]) {
        let Self { delete_state, inner, .. } = self;
        let mut buf = Vec::new();
        key.serialize(&mut buf).expect("failed to serialize key");
        let params = inner.allocate(buf);
        let ctx = inner.allocate_context();

        delete_state
            .call(inner.store_mut(), ctx)
            .expect("failed to call `delete_state` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("delete_state should always return something");

        <()>::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }

    #[inline]
    pub fn get_balance(&mut self, address: &WasmlAddress) -> u64 {
        let Self { get_balance, inner, .. } = self;
        let mut buf = Vec::new();
        address.serialize(&mut buf).expect("failed to serialize address");
        let params = inner.allocate(buf);
        let ctx = inner.allocate_context();

        get_balance
            .call(inner.store_mut(), ctx)
            .expect("failed to call `get_balance` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("get_balance should always return something");

        u64::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }

    #[inline]
    pub fn set_balance(&mut self, address: &WasmlAddress, amount: u64) {
        let Self { set_balance, inner, .. } = self;
        let mut buf = Vec::new();
        address.serialize(&mut buf).expect("failed to serialize address");
        amount.serialize(&mut buf).expect("failed to serialize amount");
        let params = inner.allocate(buf);
        let ctx = inner.allocate_context();

        set_balance
            .call(inner.store_mut(), ctx)
            .expect("failed to call `set_balance` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("set_balance should always return something");

        <()>::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }

    #[inline]
    pub fn emit_event(&mut self, event: &[u8]) {
        let Self { emit_event, inner, .. } = self;
        let params = inner.allocate(event.to_vec());
        let ctx = inner.allocate_context();

        emit_event
            .call(inner.store_mut(), ctx)
            .expect("failed to call `emit_event` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("emit_event should always return something");

        <()>::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }

    #[inline]
    pub fn get_events(&mut self) -> Vec<Vec<u8>> {
        let Self { get_events, inner, .. } = self;
        let ctx = inner.allocate_context();

        get_events
            .call(inner.store_mut(), ctx)
            .expect("failed to call `get_events` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("get_events should always return something");

        Vec::<Vec<u8>>::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }

    #[inline]
    pub fn store_schema(&mut self, schema: &[u8]) {
        let Self { store_schema, inner, .. } = self;
        let params = inner.allocate(schema.to_vec());
        let ctx = inner.allocate_context();

        store_schema
            .call(inner.store_mut(), ctx)
            .expect("failed to call `store_schema` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("store_schema should always return something");

        <()>::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }

    #[inline]
    pub fn get_schema(&mut self) -> Vec<u8> {
        let Self { get_schema, inner, .. } = self;
        let ctx = inner.allocate_context();

        get_schema
            .call(inner.store_mut(), ctx)
            .expect("failed to call `get_schema` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("get_schema should always return something");

        Vec::<u8>::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }
}

pub struct Nft {
    inner: TestCrate,
    mint: UserDefinedFn,
}

impl Nft {
    #[inline]
    pub fn new(builder: Builder) -> Self {
        let mut inner = builder.build();
        let mint = inner.get_user_defined_typed_func("mint");

        Self { inner, mint }
    }

    #[inline]
    pub fn mint(&mut self, address: WasmlAddress, id: u64) {
        let Self { mint, inner } = self;
        let mut buf = Vec::new();
        address.serialize(&mut buf).expect("failed to serialize address");
        id.serialize(&mut buf).expect("failed to serialize id");
        let params = inner.allocate(buf);

        mint.call(inner.store_mut(), params)
            .expect("failed to call `mint` function");

        let result = inner
            .store_mut()
            .data_mut()
            .take_result()
            .expect("mint should always return something");

        <()>::deserialize(&mut &result[..]).expect("failed to deserialize result")
    }
}
