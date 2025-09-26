use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Expr, Token};

#[proc_macro_attribute]
pub fn retry_on_error(attr: TokenStream, item: TokenStream) -> TokenStream {
    let retries = if attr.is_empty() {
        1
    } else {
        let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
        let args = parser.parse(attr).expect("Invalid syntax");
        match args.first() {
            Some(Expr::Lit(expr_lit)) => match &expr_lit.lit {
                syn::Lit::Int(n) => n.base10_parse::<usize>().unwrap_or(1),
                _ => 1,
            },
            _ => 1,
        }
    };

    let input = parse_macro_input!(item as ItemFn);
    let sig = &input.sig;
    let block = &input.block;
    let attrs = &input.attrs;
    let vis = &input.vis;

    let output = quote! {
        #(#attrs)*
        #vis #sig {
            let mut attempts = 0;
            loop {
                let result = (|| #block )();
                match result {
                    Ok(val) => return Ok(val),
                    Err(e) => {
                        attempts += 1;
                        if attempts >= #retries {
                            return Err(e);
                        }
                    }
                }
            }
        }
    };

    output.into()
}