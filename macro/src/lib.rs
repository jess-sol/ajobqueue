use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

#[proc_macro_attribute]
pub fn job_type(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    job_type_macro::expand(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn job(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let attrs = parse_macro_input!(attr as job_macro::JobAttrs);

    job_macro::expand(attrs, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

// Re-export typetag macros
use quote::format_ident;
use typetag_impl::{Mode, expand, get_crate_path};

#[proc_macro_attribute]
pub fn serde(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut crate_path = get_crate_path("ajobqueue");
    crate_path.segments.push(format_ident!("typetag").into());
    expand(args, input, Mode::new(true, true), &crate_path)
}

#[proc_macro_attribute]
pub fn serialize(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut crate_path = get_crate_path("ajobqueue");
    crate_path.segments.push(format_ident!("typetag").into());
    expand(args, input, Mode::new(true, false), &crate_path)
}

#[proc_macro_attribute]
pub fn deserialize(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut crate_path = get_crate_path("ajobqueue");
    crate_path.segments.push(format_ident!("typetag").into());
    expand(args, input, Mode::new(false, true), &crate_path)
}


mod job_type_macro {
    use proc_macro2::TokenStream;
    use quote::{quote, format_ident};
    use syn::{Data, DeriveInput, Result};

    pub(crate) fn expand(input: DeriveInput) -> Result<TokenStream> {
        let visibility = input.vis;
        let name = input.ident;
        let trait_name = format_ident!("{}Marker", name);
        let attrs = input.attrs;

        // TODO - Make configurable with attr
        let job_type_str = name.to_string();

        let fields = if let Data::Struct(x) = input.data {
            x.fields
        } else {
            return Err(syn::Error::new(name.span(), "Invalid type, must be struct"));
        };

        let expanded = quote! {
            #(#attrs)*
            #visibility struct #name #fields

            impl ::ajobqueue::JobType for #name {
                fn job_type() -> String {
                    String::from(#job_type_str)
                }
            }

            #[::ajobqueue::serde(tag="type")]
            #visibility trait #trait_name: ::ajobqueue::Job<JobTypeData=#name> {
                fn into_any(self: Box<Self>) -> Box<dyn ::std::any::Any>;
            }

            impl ::ajobqueue::JobTypeMarker for dyn #trait_name<JobTypeData=#name> {}
        };

        Ok(expanded)
    }
}

mod job_macro {
    use proc_macro2::{TokenStream, Ident};
    use quote::{quote, format_ident};
    use syn::{Data, DeriveInput, Result, parse::Parse};

    pub struct JobAttrs {
        pub name: Ident,
    }

    impl Parse for JobAttrs {
        fn parse(input: syn::parse::ParseStream) -> Result<Self> {
            let name: Ident = input.parse()?;
            Ok(JobAttrs { name })
        }
    }

    pub(crate) fn expand(attrs: JobAttrs, input: DeriveInput) -> Result<TokenStream> {
        let visibility = input.vis;
        let name = input.ident;
        let sattrs = input.attrs;

        let job_trait_name = format_ident!("{}Marker", attrs.name);

        let fields = if let Data::Struct(x) = input.data {
            x.fields
        } else {
            return Err(syn::Error::new(name.span(), "Invalid type, must be struct"));
        };

        let expanded = quote! {
            #(#sattrs)*
            #[derive(Clone, Debug, ::ajobqueue::typetag::serde::Serialize, ::ajobqueue::typetag::serde::Deserialize)]
            #visibility struct #name #fields

            #[::ajobqueue::serde]
            impl #job_trait_name for #name {
                fn into_any(self: Box<Self>) -> Box<dyn ::std::any::Any> {
                    self
                }
            }
        };

        Ok(expanded)
    }
}
