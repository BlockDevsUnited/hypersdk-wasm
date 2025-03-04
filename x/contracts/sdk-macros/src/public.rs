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
    let _context_pat_type = match inputs.next() {
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

    // Important: Clone the input to avoid the partial move
    let input_clone = input.clone();
    
    // Extract return type and attrs
    let ret_type = &input.sig.output;
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

    // Generate parameter indices for deserializing parameters
    let _indices: Vec<proc_macro2::TokenStream> = (0..other_inputs.len())
        .map(|i| {
            let idx = proc_macro2::Literal::usize_unsuffixed(i);
            quote! { #idx }
        })
        .collect();

    // Generate parameter types for deserializing
    let param_types: Vec<_> = other_inputs.iter().map(|pt| &pt.ty).collect();

    let wasm_result = if is_async {
        quote! {
            let result: Result<#ret_type, ::wasmlanche::Error> = futures::executor::block_on(async {
                let args_slice = unsafe {
                    let ptr = args as *const u8;
                    let len = *(ptr.offset(-4) as *const u32) as usize;
                    core::slice::from_raw_parts(ptr, len)
                };

                let mut ctx = Context::new();
                
                // Simple parameter parsing - each parameter is a separate Borsh-serialized object
                let mut param_index = 0;
                #(
                    // Check if we have enough data left
                    if param_index >= args_slice.len() {
                        return Err(::wasmlanche::Error::Serialization(
                            alloc::string::String::from("Not enough parameter data in args_slice")
                        ));
                    }
                    
                    // Get the length of the next parameter
                    let param_len = if param_index + 4 <= args_slice.len() {
                        let mut len_bytes = [0u8; 4];
                        len_bytes.copy_from_slice(&args_slice[param_index..param_index+4]);
                        u32::from_le_bytes(len_bytes) as usize
                    } else {
                        return Err(::wasmlanche::Error::Serialization(
                            alloc::string::String::from("Failed to read parameter length")
                        ));
                    };
                    
                    // Move past the length
                    param_index += 4;
                    
                    // Check if we have enough data for this parameter
                    if param_index + param_len > args_slice.len() {
                        return Err(::wasmlanche::Error::Serialization(
                            alloc::string::String::from("Parameter data exceeds available bytes")
                        ));
                    }
                    
                    // Get the parameter bytes
                    let param_bytes = &args_slice[param_index..param_index + param_len];
                    
                    // Move past this parameter
                    param_index += param_len;
                    
                    // Deserialize the parameter
                    let #param_names = match <#param_types as ::borsh::BorshDeserialize>::try_from_slice(param_bytes) {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(::wasmlanche::Error::Serialization(
                                alloc::string::String::from(alloc::format!("Failed to deserialize parameter: {}", err))
                            ));
                        }
                    };
                )*

                Ok(#function_call)
            });
        }
    } else {
        quote! {
            let result: Result<#ret_type, ::wasmlanche::Error> = {
                let args_slice = unsafe {
                    let ptr = args as *const u8;
                    let len = *(ptr.offset(-4) as *const u32) as usize;
                    core::slice::from_raw_parts(ptr, len)
                };

                let mut ctx = Context::new();
                
                // Simple parameter parsing - each parameter is a separate Borsh-serialized object
                let mut param_index = 0;
                #(
                    // Check if we have enough data left
                    if param_index >= args_slice.len() {
                        return Err(::wasmlanche::Error::Serialization(
                            alloc::string::String::from("Not enough parameter data in args_slice")
                        ));
                    }
                    
                    // Get the length of the next parameter
                    let param_len = if param_index + 4 <= args_slice.len() {
                        let mut len_bytes = [0u8; 4];
                        len_bytes.copy_from_slice(&args_slice[param_index..param_index+4]);
                        u32::from_le_bytes(len_bytes) as usize
                    } else {
                        return Err(::wasmlanche::Error::Serialization(
                            alloc::string::String::from("Failed to read parameter length")
                        ));
                    };
                    
                    // Move past the length
                    param_index += 4;
                    
                    // Check if we have enough data for this parameter
                    if param_index + param_len > args_slice.len() {
                        return Err(::wasmlanche::Error::Serialization(
                            alloc::string::String::from("Parameter data exceeds available bytes")
                        ));
                    }
                    
                    // Get the parameter bytes
                    let param_bytes = &args_slice[param_index..param_index + param_len];
                    
                    // Move past this parameter
                    param_index += param_len;
                    
                    // Deserialize the parameter
                    let #param_names = match <#param_types as ::borsh::BorshDeserialize>::try_from_slice(param_bytes) {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(::wasmlanche::Error::Serialization(
                                alloc::string::String::from(alloc::format!("Failed to deserialize parameter: {}", err))
                            ));
                        }
                    };
                )*

                Ok(#function_call)
            };
        }
    };

    // Generate the final token stream
    let output = quote! {
        // Redefine the function as a non-public function
        #(#attrs)*
        #[allow(unused_variables)]
        #input_clone

        // Define the public WASM entry point
        #[no_mangle]
        pub unsafe extern "C" fn #wasm_name(args: *const u8) -> *mut u8 {
            use ::wasmlanche::{Context, Error};
            use ::borsh::{BorshSerialize, BorshDeserialize};

            #wasm_result

            // Serialize the result or error
            let bytes = match result {
                Ok(result) => match ::borsh::BorshSerialize::try_to_vec(&result) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        let error = ::wasmlanche::Error::Serialization(
                            alloc::string::String::from(alloc::format!("Failed to serialize result: {}", e))
                        );
                        match ::borsh::BorshSerialize::try_to_vec(&error) {
                            Ok(bytes) => bytes,
                            Err(_) => {
                                alloc::vec![0u8; 0]
                            }
                        }
                    }
                },
                Err(err) => match ::borsh::BorshSerialize::try_to_vec(&err) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        alloc::vec![0u8; 0]
                    }
                },
            };

            // Copy result to WASM heap
            let result_ptr = ::wasmlanche::allocate(bytes.len() as u32);
            unsafe {
                let result_slice = core::slice::from_raw_parts_mut(result_ptr, bytes.len());
                result_slice.copy_from_slice(&bytes);
            }
            result_ptr
        }
    };

    Ok(output)
}
