// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};

mod public;

/// The `public` attribute macro makes a function an entry-point for your smart-contract.
/// The function must have `pub` visibility and take a mutable reference to `Context` as its first parameter.
/// Additional parameters must implement `BorshSerialize` + `BorshDeserialize`.
/// The return type must also implement `BorshSerialize` + `BorshDeserialize`.
#[proc_macro_attribute]
pub fn public(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    match public::impl_public(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
