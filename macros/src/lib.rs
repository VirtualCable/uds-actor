use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Expr, Token};
use syn::{ItemFn, parse_macro_input};

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
            let mut backoff = std::time::Duration::from_millis(500);
            loop {
                let result = (|| #block )();
                match result {
                    Ok(val) => return Ok(val),
                    Err(e) => {
                        shared::log::info!("Operation failed: {}. Retrying... (attempt {}/{})", e, attempts + 1, #retries);
                        attempts += 1;
                        if attempts >= #retries {
                            return Err(e);
                        }
                        std::thread::sleep(backoff);
                        if backoff < std::time::Duration::from_secs(8) {
                            backoff *= 2; // Exponential backoff, with a max of 8 seconds
                        }
                    }
                }
            }
        }
    };

    output.into()
}
