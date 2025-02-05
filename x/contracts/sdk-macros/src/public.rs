// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote, parse_str,
    punctuated::Punctuated,
    spanned::Spanned,
    Block, Error, FnArg, Generics, Ident, ItemFn, Pat, PatIdent, PatType, PatWild, ReturnType,
    Signature, Token, Type, TypeReference, Visibility,
};

const CONTEXT_TYPE: &str = "&mut wasmlanche::Context";

type CommaSeparated<T> = Punctuated<T, Token![,]>;

pub fn impl_public(public_fn: PublicFn) -> Result<TokenStream, Error> {
    let args_names = public_fn
        .sig
        .other_inputs
        .iter()
        .map(|PatType { pat: name, .. }| quote! {#name});
    let args_names_2 = args_names.clone();

    let name = &public_fn.sig.ident;
    let context_type = type_from_reference(&public_fn.sig.user_defined_context_type);

    let other_inputs = public_fn.sig.other_inputs.iter();

    let external_call = quote! {
        mod private {
            use super::*;
            #[derive(wasmlanche::borsh::BorshDeserialize)]
            #[borsh(crate = "wasmlanche::borsh")]
            struct Args {
                ctx: #context_type,
                #(#other_inputs),*
            }

            #[link(wasm_import_module = "contract")]
            extern "C" {
                #[link_name = "set_call_result"]
                fn set_call_result(ptr: *const u8, len: usize);
            }

            #[cfg(target_arch = "wasm32")]
            unsafe fn get_args_slice(ptr: wasmlanche::HostPtr) -> &'static [u8] {
                let ptr = ptr as *const u8;
                let len = *(ptr.offset(-4) as *const u32) as usize;
                std::slice::from_raw_parts(ptr, len)
            }

            #[no_mangle]
            unsafe extern "C-unwind" fn #name(args: wasmlanche::HostPtr) {
                wasmlanche::register_panic();

                let result = {
                    #[cfg(target_arch = "wasm32")]
                    let args_slice = get_args_slice(args);
                    #[cfg(not(target_arch = "wasm32"))]
                    let args_slice = &args;

                    let args: Args = wasmlanche::borsh::from_slice(args_slice).expect("error fetching serialized args");

                    let Args { mut ctx, #(#args_names),* } = args;

                    let result = super::#name(&mut ctx, #(#args_names_2),*);
                    wasmlanche::borsh::to_vec(&result).expect("error serializing result")
                };

                unsafe { set_call_result(result.as_ptr(), result.len()) };
            }
        }
    };

    let mut binding_fn = public_fn.to_bindings_fn()?;

    let feature_name = "bindings";

    let mut public_fn = public_fn;

    public_fn
        .attrs
        .push(parse_quote! { #[cfg(not(feature = #feature_name))] });
    binding_fn
        .attrs
        .push(parse_quote! { #[cfg(feature = #feature_name)] });

    public_fn
        .block
        .stmts
        .insert(0, syn::parse2(external_call).unwrap());

    let public_fn = ItemFn::from(public_fn);

    let result = quote! {
        #binding_fn
        #public_fn
    };

    Ok(result)
}

pub struct PublicFn {
    attrs: Vec<syn::Attribute>,
    vis: Visibility,
    sig: PublicFnSignature,
    block: Box<Block>,
}

impl Parse for PublicFn {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let input = ItemFn::parse(input)?;

        let ItemFn {
            attrs,
            vis,
            sig,
            block,
        } = input;

        let vis_err = if !matches!(&vis, Visibility::Public(_)) {
            let err = Error::new(
                sig.span(),
                "Functions with the `#[public]` attribute must have `pub` visibility.",
            );

            Some(err)
        } else {
            None
        };

        let mut inputs = sig.inputs.into_iter();
        let paren_span = sig.paren_token.span.join();

        let (user_defined_context_type, context_input) =
            extract_context_arg(&mut inputs, paren_span);

        let context_input = match (vis_err, context_input) {
            (Some(mut vis_err), Err(context_err)) => {
                vis_err.combine(context_err);
                Err(vis_err)
            }
            (Some(vis_err), Ok(_)) => Err(vis_err),
            (None, context_input) => context_input,
        };

        let other_inputs = map_other_inputs(inputs);

        let (_context_input, other_inputs) = match (context_input, other_inputs) {
            (Err(mut vis_and_first), Err(rest)) => {
                vis_and_first.combine(rest);
                Err(vis_and_first)
            }
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
            (Ok(context_input), Ok(other_inputs)) => Ok((context_input, other_inputs)),
        }?;

        let fn_token = sig.fn_token;

        let sig = PublicFnSignature {
            fn_token,
            ident: sig.ident,
            generics: sig.generics,
            user_defined_context_type,
            other_inputs,
            output: sig.output,
        };

        Ok(Self {
            attrs,
            vis,
            sig,
            block,
        })
    }
}

impl From<PublicFn> for ItemFn {
    fn from(public_fn: PublicFn) -> Self {
        let PublicFn {
            attrs,
            vis,
            sig,
            block,
        } = public_fn;

        Self {
            attrs,
            vis,
            sig: sig.into(),
            block,
        }
    }
}

#[derive(Clone)]
pub struct PublicFnSignature {
    pub fn_token: Token![fn],
    pub ident: Ident,
    pub generics: Generics,
    pub user_defined_context_type: Box<Type>,
    pub other_inputs: CommaSeparated<PatType>,
    pub output: ReturnType,
}

impl From<PublicFnSignature> for Signature {
    fn from(value: PublicFnSignature) -> Self {
        let PublicFnSignature {
            fn_token,
            ident,
            generics,
            user_defined_context_type,
            other_inputs,
            output,
        } = value;

        let mut inputs = CommaSeparated::new();
        inputs.push(FnArg::Typed(PatType {
            attrs: vec![],
            pat: Box::new(Pat::Ident(PatIdent {
                attrs: vec![],
                by_ref: None,
                mutability: None,
                ident: format_ident!("context"),
                subpat: None,
            })),
            colon_token: Token![:](Span::call_site()),
            ty: user_defined_context_type,
        }));

        for input in other_inputs {
            inputs.push(FnArg::Typed(input));
        }

        Self {
            constness: None,
            asyncness: None,
            unsafety: None,
            abi: None,
            fn_token,
            ident,
            generics,
            paren_token: syn::token::Paren(Span::call_site()),
            inputs,
            variadic: None,
            output,
        }
    }
}

#[derive(Clone)]
pub struct ContextArg {
    pub attrs: Vec<syn::Attribute>,
    pub pat: Box<Pat>,
    pub colon_token: Token![:],
    pub ty: Box<Type>,
}

impl From<ContextArg> for FnArg {
    fn from(value: ContextArg) -> Self {
        let ContextArg {
            attrs,
            pat,
            colon_token,
            ty,
        } = value;

        FnArg::Typed(PatType {
            attrs,
            pat,
            colon_token,
            ty,
        })
    }
}

impl PublicFn {
    fn to_bindings_fn(&self) -> Result<ItemFn, Error> {
        let sig = &self.sig;

        let name = &sig.ident;
        let other_inputs = sig.other_inputs.iter().collect::<Vec<_>>();
        let args_names = sig
            .other_inputs
            .iter()
            .map(|PatType { pat: name, .. }| quote! {#name})
            .collect::<Vec<_>>();

        let context_type = type_from_reference(&sig.user_defined_context_type);

        let bindings_fn = quote! {
            pub fn #name(
                context: &mut wasmlanche::ExternalCallContext,
                #(#other_inputs),*
            ) -> Result<_, wasmlanche::ExternalCallError> {
                let args = {
                    #[derive(wasmlanche::borsh::BorshSerialize)]
                    #[borsh(crate = "wasmlanche::borsh")]
                    struct Args {
                        ctx: #context_type,
                        #(#other_inputs),*
                    }

                    Args {
                        ctx: wasmlanche::Context::new(),
                        #(#args_names),*
                    }
                };

                let result = context.execute_wasm(
                    wasmlanche::borsh::to_vec(&args)?.as_slice(),
                    stringify!(#name),
                )?;

                wasmlanche::borsh::from_slice(&result).map_err(Into::into)
            }
        };

        syn::parse2(bindings_fn)
    }
}

fn extract_context_arg<I>(inputs: &mut I, paren_span: Span) -> (Box<Type>, Result<ContextArg, Error>)
where
    I: Iterator<Item = FnArg>,
{
    let Some(context_arg) = inputs.next() else {
        return (
            Box::new(parse_str(CONTEXT_TYPE).unwrap()),
            Err(Error::new(paren_span, "missing context argument")),
        );
    };

    let FnArg::Typed(PatType { pat, ty, .. }) = context_arg else {
        return (
            Box::new(parse_str(CONTEXT_TYPE).unwrap()),
            Err(Error::new(
                context_arg.span(),
                "self argument is not allowed in public functions",
            )),
        );
    };

    if !is_mutable_context_ref(&ty) {
        return (
            Box::new(parse_str(CONTEXT_TYPE).unwrap()),
            Err(Error::new(
                ty.span(),
                "context argument must be a mutable reference to Context",
            )),
        );
    }

    (
        ty.clone(),
        Ok(ContextArg {
            attrs: vec![],
            pat: pat.clone(),
            colon_token: Token![:](Span::call_site()),
            ty: ty.clone(),
        }),
    )
}

fn map_other_inputs(inputs: impl Iterator<Item = FnArg>) -> Result<CommaSeparated<PatType>, Error> {
    let mut other_inputs = CommaSeparated::new();

    for input in inputs {
        let FnArg::Typed(pat_type) = input else {
            return Err(Error::new(
                input.span(),
                "self argument is not allowed in public functions",
            ));
        };

        let PatType { pat, ty, .. } = pat_type;

        let pat_clone = pat.clone();
        match &*pat {
            Pat::Ident(PatIdent { mutability, .. }) => {
                if mutability.is_some() {
                    return Err(Error::new(
                        mutability.span(),
                        "mutable arguments are not allowed in public functions",
                    ));
                }
            }
            Pat::Wild(_) => {}
            _ => {
                return Err(Error::new(
                    pat.span(),
                    "pattern matching is not allowed in public functions",
                ));
            }
        }

        other_inputs.push(PatType {
            attrs: vec![],
            pat: pat_clone,
            colon_token: Token![:](Span::call_site()),
            ty,
        });
    }

    Ok(other_inputs)
}

/// Returns whether the type_path represents a mutable context ref type.
fn is_mutable_context_ref(type_path: &Type) -> bool {
    let Type::Reference(TypeReference {
        mutability: Some(_),
        elem,
        ..
    }) = type_path
    else {
        return false;
    };

    let Type::Path(type_path) = &**elem else {
        return false;
    };

    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };

    segment.ident == "Context"
}

fn type_from_reference(type_path: &Type) -> &Type {
    let Type::Reference(TypeReference { elem, .. }) = type_path else {
        unreachable!()
    };

    elem
}
