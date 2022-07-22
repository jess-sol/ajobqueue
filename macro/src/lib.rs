use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

#[proc_macro_attribute]
pub fn executor(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{}\"", attr);
    println!("item: \"{}\"", item);

    let input = parse_macro_input!(item as DeriveInput);

    ExecutorMacro::expand(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

mod ExecutorMacro {
    use proc_macro2::TokenStream;
    use quote::{quote, format_ident};
    use syn::{Data, DeriveInput, Result};

    pub(crate) fn expand(input: DeriveInput) -> Result<TokenStream> {
        let visibility = input.vis;
        let name = input.ident;
        let modname = format_ident!("{}Impl", name);

        let fields = if let Data::Struct(x) = input.data {
            x.fields
        } else {
            return Err(syn::Error::new(name.span(), "Invalid type, must be struct"));
        };

        let expanded = quote! {
            #visibility struct #name #fields
            #[allow(non_snake_case)]
            mod #modname {
                use super::#name;
                use super::MsgJobFamily; // TODO
                use crate::{Job, Executor};
                use linkme::distributed_slice;

                type JobFamily = MsgJobFamily; // TODO

                #[distributed_slice]
                pub(super) static JOBS: [fn() -> Box<dyn Job<JobFamily=JobFamily>>] = [..];

                // impl Executor for #name {
                //     type JobFamily = JobFamily;

                //     fn new(worker_data: Self::JobFamily) -> Self {
                //         // let x = JOBS;
                //         // println!("HIDER: {}", ty!(JOBS));
                //         // println!("HIDER: {:?}", JOBS!());
                //         Self { worker_data }
                //     }

                //     fn worker_data(&self) -> &Self::JobFamily {
                //         &self.worker_data
                //     }
                // }
            }
        };

        println!("HIDER: {}", expanded);

        Ok(expanded)
    }

    fn expand_distributed_slice_enum(input: TokenStream) -> Result<TokenStream> {
        Ok(input)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
