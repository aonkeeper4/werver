use itertools::join;
use proc_macro::{self, TokenStream};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    parse_macro_input, FnArg, Ident, ItemFn, LitStr, Pat, PatIdent, PatType, Token, Type,
    TypeReference,
};

#[allow(dead_code)]
struct RouteMeta {
    request_type: Ident,
    comma_1: Token![,],
    prefixes: Punctuated<LitStr, Token![|]>,
}

impl Parse for RouteMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            request_type: input.parse()?,
            comma_1: input.parse()?,
            prefixes: Punctuated::parse_terminated(input)?,
        })
    }
}

fn expand_route(attr: &RouteMeta, input: &ItemFn) -> syn::Result<TokenStream2> {
    // add full route w/ args in incorrect args error
    let name = &input.sig.ident;
    let inputs = &input.sig.inputs;
    let num_inputs = inputs.len();
    let vis = &input.vis;

    let RouteMeta {
        request_type,
        prefixes,
        ..
    } = attr;
    let route_prefix = match prefixes.first() {
        Some(v) => v.value(),
        None => {
            return Err(syn::Error::new_spanned(
                prefixes,
                "must have one or more route prefixes",
            ))
        }
    };
    let prefixes_vec: Vec<_> = prefixes.iter().map(LitStr::value).collect();

    let args = inputs
        .iter()
        .map(|arg| match arg {
            FnArg::Receiver(_) => Err(syn::Error::new_spanned(
                arg,
                "this macro does not support functions that take a `self` argument",
            )),
            FnArg::Typed(PatType { ty, pat, .. }) => {
                let Pat::Ident(PatIdent {
                    ident: arg_name, ..
                }) = pat.as_ref()
                else {
                    return Err(syn::Error::new_spanned(
                        pat,
                        "this macro does not support pattern matching in the fn arguments",
                    ));
                };
                Ok((arg_name, ty))
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let arg_names: Vec<_> = args
        .iter()
        .map(|(arg_name, _)| format!("{{{arg_name}}}"))
        .collect();
    let args_without_types: Vec<_> = args.iter().map(|(arg, _)| arg).collect();
    let route_str = route_prefix + "/" + &join(arg_names, "/");

    let parse_inputs: TokenStream2 = args
        .iter()
        .enumerate()
        .map(|(i, (arg_name, ty))| {
            let arg_name_str = arg_name.to_string();
            if let Type::Reference(TypeReference { elem, .. }) = ty.as_ref() {
                quote! {
                    let ref #arg_name = match args[#i].parse::<#elem>() {
                        Ok(x) => x,
                        Err(e) => {
                            return Err(format!(
                                "Failed to parse argument `{}` in route `{}`: {}",
                            #arg_name_str, #route_str, e))
                        },
                    };
                }
            } else {
                quote! {
                    let #arg_name = match args[#i].parse::<#ty>() {
                        Ok(x) => x,
                        Err(e) => {
                            return Err(format!(
                                "Failed to parse argument `{}` in route `{}`: {}",
                            #arg_name_str, #route_str, e))
                        },
                    };
                }
            }
        })
        .collect();

    let result = quote! {
            #[allow(non_camel_case_types)]
            #vis struct #name;

            // basically just an expanded lazy_static!
            impl std::ops::Deref for #name {
            type Target = crate::http_server::Route;

            fn deref(&self) -> &Self::Target {
                static ONCE: std::sync::Once = std::sync::Once::new();
                static mut VALUE: *mut crate::http_server::Route = 0 as *mut crate::http_server::Route;

                unsafe {
                    ONCE.call_once(|| VALUE = Box::into_raw(Box::new(crate::http_server::Route::new(
                        crate::http_server::RequestType::#request_type,
                        vec![#(#prefixes_vec.to_string()),*],
                        |args| {
                            if args.len() != #num_inputs {
                                return Err(format!("Incorrect number of arguments given (expected {}, got {})", #num_inputs, args.len()));
                            }

                            #parse_inputs

                            #[allow(clippy::unnecessary_wraps)]
                            #input
                            #name(#(#args_without_types),*)
                        },
                    ))));

                    &*VALUE
                }
            }
        }



    };
    Ok(result)
}

#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as RouteMeta);
    let input = parse_macro_input!(item as ItemFn);
    expand_route(&attr, &input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
