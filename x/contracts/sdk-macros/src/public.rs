// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, PatType, Type, TypeReference, Visibility};

pub fn impl_public(input: ItemFn) -> Result<TokenStream, syn::Error> {
    // Validate function visibility
    if !matches!(&input.vis, Visibility::Public(_)) {
        return Err(syn::Error::new_spanned(
            &input.sig,
            "Functions with the `#[public]` attribute must have `pub` visibility.",
        ));
    }

    let name = &input.sig.ident;
    let wasm_name = quote::format_ident!("__wasm_{}", name);
    let mut inputs = input.sig.inputs.iter().cloned();
    let is_async = input.sig.asyncness.is_some();

    // Extract and validate context parameter
    let context_pat_type = match inputs.next() {
        Some(FnArg::Typed(pat_type)) => {
            if let Type::Reference(TypeReference {
                mutability: Some(_),
                elem,
                ..
            }) = &*pat_type.ty
            {
                if let Type::Path(type_path) = &**elem {
                    if let Some(segment) = type_path.path.segments.last() {
                        if segment.ident == "Context" {
                            pat_type
                        } else {
                            return Err(syn::Error::new_spanned(
                                &pat_type.ty,
                                "First argument must be a mutable reference to Context",
                            ));
                        }
                    } else {
                        return Err(syn::Error::new_spanned(
                            &pat_type.ty,
                            "First argument must be a mutable reference to Context",
                        ));
                    }
                } else {
                    return Err(syn::Error::new_spanned(
                        &pat_type.ty,
                        "First argument must be a mutable reference to Context",
                    ));
                }
            } else {
                return Err(syn::Error::new_spanned(
                    &pat_type.ty,
                    "First argument must be a mutable reference to Context",
                ));
            }
        }
        Some(_) => {
            return Err(syn::Error::new_spanned(
                &input.sig,
                "First argument must be a mutable reference to Context",
            ))
        }
        None => {
            return Err(syn::Error::new_spanned(
                &input.sig,
                "Function must take a mutable reference to Context as its first argument",
            ))
        }
    };

    // Collect remaining parameters
    let other_inputs: Vec<PatType> = inputs
        .filter_map(|arg| match arg {
            FnArg::Typed(pat_type) => Some(pat_type),
            _ => None,
        })
        .collect();

    // Generate parameter names for the function call
    let param_names: Vec<_> = other_inputs
        .iter()
        .map(|pat_type| &*pat_type.pat)
        .collect();

    // Generate the public function
    let block = input.block;
    let ret_type = input.sig.output;
    let attrs = &input.attrs;

    let function_call = if is_async {
        quote! {
            super::#name(&mut ctx, #(#param_names),*).await
        }
    } else {
        quote! {
            super::#name(&mut ctx, #(#param_names),*)
        }
    };

    let async_token = if is_async {
        quote! { async }
    } else {
        quote! {}
    };

    let wasm_async_token = if is_async {
        quote! { async }
    } else {
        quote! {}
    };

    let wasm_result = if is_async {
        quote! {
            let result = futures::executor::block_on(async {
                let args_slice = unsafe {
                    let ptr = args as *const u8;
                    let len = *(ptr.offset(-4) as *const u32) as usize;
                    core::slice::from_raw_parts(ptr, len)
                };

                let mut ctx = Context::new();
                
                let mut offset = core::mem::size_of::<Context>();
                
                #(
                    let #param_names = if offset < args_slice.len() {
                        let param_bytes = &args_slice[offset..];
                        match ::borsh::from_slice(param_bytes) {
                            Ok(val) => {
                                // Update offset for next parameter
                                offset += ::borsh::serialized_size(&val).unwrap_or(0);
                                val
                            },
                            Err(_) => Default::default()
                        }
                    } else {
                        Default::default()
                    };
                )*

                #function_call
            });
        }
    } else {
        quote! {
            let result = {
                let args_slice = unsafe {
                    let ptr = args as *const u8;
                    let len = *(ptr.offset(-4) as *const u32) as usize;
                    core::slice::from_raw_parts(ptr, len)
                };

                let mut ctx = Context::new();
                
                let mut offset = core::mem::size_of::<Context>();
                
                #(
                    let #param_names = if offset < args_slice.len() {
                        let param_bytes = &args_slice[offset..];
                        match ::borsh::from_slice(param_bytes) {
                            Ok(val) => {
                                // Update offset for next parameter
                                offset += ::borsh::serialized_size(&val).unwrap_or(0);
                                val
                            },
                            Err(_) => Default::default()
                        }
                    } else {
                        Default::default()
                    };
                )*

                #function_call
            };
        }
    };

    Ok(quote! {
        #(#attrs)*
        #[cfg_attr(target_arch = "wasm32", no_mangle)]
        pub #async_token fn #name(#context_pat_type, #(#other_inputs),*) #ret_type {
            #block
        }

        #[cfg(target_arch = "wasm32")]
        mod __wasm_exports {
            use super::*;

            pub struct Args {
                pub ctx: Context,
                #(pub #other_inputs),*
            }

            #[no_mangle]
            pub unsafe extern "C-unwind" fn #wasm_name(args: u32) -> i64 {
                // Panic handling is now managed by the wasmlanche runtime
                // No need to call register_panic here

                #wasm_result

                let result_bytes = ::borsh::to_vec(&result)
                    .expect("Failed to serialize result");

                let ptr = result_bytes.as_ptr() as i64;
                let len = result_bytes.len() as i64;
                (ptr << 32) | len
            }
        }
    })
}
