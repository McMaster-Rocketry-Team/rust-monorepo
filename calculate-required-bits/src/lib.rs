use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, LitFloat, LitStr, Token};

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
    let Args {
        min,
        max,
        max_error,
    } = parse_macro_input!(input as Args);

    let range = max - min;
    let bits = (range / max_error).log2().ceil() as usize;

    let expanded = quote! {
        #bits
    };

    TokenStream::from(expanded)
}

struct DocStrArgs {
    min: f32,
    max: f32,
    max_error: f32,
    name: Ident,
}

impl Parse for DocStrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let min: LitFloat = input.parse()?;
        input.parse::<Token![,]>()?;
        let max: LitFloat = input.parse()?;
        input.parse::<Token![,]>()?;
        let max_error: LitFloat = input.parse()?;
        input.parse::<Token![,]>()?;
        let name: Ident = input.parse()?;

        Ok(DocStrArgs {
            min: min.base10_parse()?,
            max: max.base10_parse()?,
            max_error: max_error.base10_parse()?,
            name,
        })
    }
}

#[proc_macro]
pub fn calculate_required_bits_docstr(input: TokenStream) -> TokenStream {
    let DocStrArgs {
        min,
        max,
        max_error,
        name,
    } = parse_macro_input!(input as DocStrArgs);

    let range = max - min;
    let bits = (range / max_error).log2().ceil() as usize;
    let docstr = format!(
        "Bits: {}, Range: {} ~ {}, Max Error: {}",
        bits, min, max, max_error
    );
    let docstr = docstr.as_str();

    let expanded = quote! {
        #[doc = #docstr]
        pub struct #name;
    };

    TokenStream::from(expanded)
}
