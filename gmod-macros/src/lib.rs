#[macro_use]
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::ItemFn;
use syn::Pat;
use syn::PatIdent;

macro_rules! wrap_compile_error {
    ($input:ident, $code:expr) => {{
        let orig_tokens = $input.clone();
        match (|| -> Result<TokenStream, syn::Error> { $code })() {
            Ok(tokens) => tokens,
            Err(_) => return orig_tokens,
        }
    }};
}

fn parse_lua_ident(input: &syn::FnArg) -> syn::Ident {
    match input {
        syn::FnArg::Typed(arg) => {
            if let Pat::Ident(PatIdent { ident, .. }) = &*arg.pat {
                ident.clone()
            } else {
                panic!("Can't use self for lua functions!")
            }
        }
        syn::FnArg::Receiver(_) => panic!("Needs to be a lua state!"),
    }
}

fn check_lua_function(input: &mut ItemFn) {
    assert!(input.sig.asyncness.is_none(), "Cannot be async");
    assert!(input.sig.constness.is_none(), "Cannot be const");
    assert!(input.sig.inputs.len() == 1, "There can only be one argument, and it should be a pointer to the Lua state (gmod::lua::State)");
    assert!(
        input.sig.abi.is_none()
            || input
                .sig
                .abi
                .as_ref()
                .and_then(|abi| abi.name.as_ref())
                .map(|abi| abi.value() == "C-unwind")
                .unwrap_or(true),
        "Do not specify an ABI"
    );
    input.sig.abi = Some(syn::parse_quote!(extern "C-unwind"));
}

fn genericify_return(item_fn: &mut ItemFn) -> proc_macro2::TokenStream {
    // let stmts = std::mem::take(&mut item_fn.block.stmts);
    // let output = std::mem::replace(&mut item_fn.sig.output, parse_quote!(-> i32));

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = item_fn.clone();

    let syn::Signature {
        output: return_type,
        inputs,
        ident: name,
        ..
    } = &sig;

    let return_type = match return_type {
        syn::ReturnType::Default => quote!(()),
        syn::ReturnType::Type(_, ty) => quote!(#ty),
    };

    let lua_ident = parse_lua_ident(&inputs[0]);

    let internal_name = syn::Ident::new(
        &format!("__{name}_internal__"),
        proc_macro2::Span::call_site(),
    );

    let output = quote! {
        #(#attrs)*
        #vis extern "C-unwind" fn #name(#inputs) -> i32
        {
            #(#attrs)*
            #[inline]
            fn #internal_name(#inputs) -> #return_type
            {
                #block
            }

            use ::gmod::lua::HandleLuaFunctionReturn;
            {
                fn assert_send<T: ::gmod::lua::HandleLuaFunctionReturn>() {}
                fn assert() {
                    assert_send::<#return_type>();
                }
            }
            #internal_name(#lua_ident).handle_result(#lua_ident)
        }
    };

    output
}

#[proc_macro_attribute]
pub fn gmod13_open(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    wrap_compile_error!(tokens, {
        let mut input = syn::parse::<ItemFn>(tokens)?;

        // Make sure it's valid
        check_lua_function(&mut input);

        let lua_ident = parse_lua_ident(&input.sig.inputs[0]);

        let block = input.block;
        input.block = syn::parse2(quote! {{
            #[allow(unused_unsafe)]
            unsafe {
                ::gmod::lua::load()
            }

            ::gmod::lua::task_queue::load(#lua_ident);

            #block
        }})
        .unwrap();

        // No mangling
        input.attrs.push(parse_quote!(#[no_mangle]));

        Ok(genericify_return(&mut input).into())
    })
}

#[proc_macro_attribute]
pub fn gmod13_close(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    wrap_compile_error!(tokens, {
        let mut input = syn::parse::<ItemFn>(tokens)?;

        // Make sure it's valid
        check_lua_function(&mut input);

        // No mangling
        input.attrs.push(parse_quote!(#[no_mangle]));

        let lua_ident = parse_lua_ident(&input.sig.inputs[0]);

        let block = input.block;
        input.block = syn::parse2(quote! {{
            ::gmod::defer!(::gmod::lua::task_queue::unload(#lua_ident)); // we should be the last thing to run

            #block
        }})
        .unwrap();

        // Make the return type nice and dynamic
        Ok(genericify_return(&mut input).into())
    })
}

#[proc_macro_attribute]
pub fn lua_function(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    wrap_compile_error!(tokens, {
        let mut input = syn::parse::<ItemFn>(tokens)?;

        // Make sure it's valid
        check_lua_function(&mut input);

        // Make the return type nice and dynamic
        Ok(genericify_return(&mut input).into())
    })
}
