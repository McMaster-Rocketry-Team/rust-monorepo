use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitFloat, Token};
use syn::parse::{Parse, ParseStream};

struct Args {
    min: f32,
    max: f32,
    max_error: f32,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let min: LitFloat = input.parse()?;
        input.parse::<Token![,]>()?;
        let max: LitFloat = input.parse()?;
        input.parse::<Token![,]>()?;
        let max_error: LitFloat = input.parse()?;

        Ok(Args {
            min: min.base10_parse()?,
            max: max.base10_parse()?,
            max_error: max_error.base10_parse()?,
        })
    }
}

#[proc_macro]
pub fn calculate_required_bits(input: TokenStream) -> TokenStream {
    let Args { min, max, max_error } = parse_macro_input!(input as Args);

    let range = max - min;
    let bits = (range / max_error).log2().ceil() as usize;

    let expanded = quote! {
        #bits
    };

    TokenStream::from(expanded)
}