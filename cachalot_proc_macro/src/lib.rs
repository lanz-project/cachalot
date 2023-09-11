use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, Signature};

use darling::{ast::NestedMeta, Error, FromMeta};
use proc_macro_error::{abort, proc_macro_error};

#[derive(Clone, Copy)]
enum FileSize {
    Bytes(u128),
    Kbs(u128),
    Mbs(u128),
    Gbs(u128),
}

impl Default for FileSize {
    fn default() -> Self {
        FileSize::Bytes(0)
    }
}

impl FileSize {
    fn size(self) -> u128 {
        match self {
            FileSize::Bytes(n) => n,
            FileSize::Kbs(n) => byte_unit::n_kib_bytes!(n),
            FileSize::Mbs(n) => byte_unit::n_mib_bytes(n),
            FileSize::Gbs(n) => byte_unit::n_gib_bytes!(n),
        }
    }
}

#[derive(FromMeta)]
struct StoreArgs {
    #[darling(default)]
    root: Option<String>,
    #[darling(default, map=FileSize::Bytes)]
    bytes: FileSize,
    #[darling(default, map=FileSize::Kbs)]
    kbs: FileSize,
    #[darling(default, map=FileSize::Mbs)]
    mbs: FileSize,
    #[darling(default, map=FileSize::Gbs)]
    gbs: FileSize,
}

impl StoreArgs {
    fn new(args: TokenStream) -> StoreArgs {
        let attr_args = match NestedMeta::parse_meta_list(args.into()) {
            Ok(v) => v,
            Err(e) => {
                abort!(Error::from(e).write_errors(), "can't parse cachalot args");
            }
        };

        match StoreArgs::from_list(&attr_args) {
            Ok(v) => v,
            Err(e) => {
                abort!(e.write_errors(), "can't parse cachalot args");
            }
        }
    }

    fn file_size(&self) -> u128 {
        [self.gbs, self.mbs, self.kbs, self.bytes]
            .map(|f| f.size())
            .into_iter()
            .sum()
    }
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn cachalot(args: TokenStream, input: TokenStream) -> TokenStream {
    transform(false, args, input)
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn try_cachalot(args: TokenStream, input: TokenStream) -> TokenStream {
    transform(true, args, input)
}

fn transform(is_try: bool, args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);

    let vis = input.vis.clone();

    let Signature {
        constness,
        asyncness,
        unsafety,
        abi,
        ident,
        generics,
        inputs,
        output,
        ..
    } = input.sig.clone();

    let (generic_params, generic_type_params, where_clause) = {
        let mut generics = generics;

        (
            generics.params.clone(),
            generics.type_params().cloned().collect::<Vec<_>>(),
            generics.where_clause.take(),
        )
    };

    if constness.is_some() {
        abort!(constness, "cachalot doesn't support const functions");
    }

    if abi.is_some() {
        abort!(abi, "cachalot doesn't support extern functions");
    }

    if asyncness.is_none() {
        abort!(asyncness, "cachalot supports async functions only");
    }

    let store_args = StoreArgs::new(args);

    let use_store = if is_try {
        quote!(
            use cachalot::TryStore;
        )
    } else {
        quote!(
            use cachalot::Store;
        )
    };

    let inner_source = {
        let mut args_types = inputs
            .iter()
            .map(|input| match input {
                FnArg::Receiver(r) => abort!(r, "cachalot doesn't support struct methods"),
                FnArg::Typed(arg) => {
                    let pat = &arg.pat;
                    let ty = &arg.ty;

                    (quote!(#pat), quote!(#ty))
                }
            })
            .collect::<Vec<_>>();

        let (range_pat, range_ty) = args_types.pop().unwrap();
        let (key_pats, key_tys): (Vec<_>, Vec<_>) = args_types.into_iter().unzip();

        let stmts = input.block.stmts;

        quote! {
            async fn #ident <#generic_params> (k: (#(#key_tys),*), #range_pat: #range_ty) #output #where_clause {
                let (#(#key_pats),*) = k;

                #(#stmts)*
            }
        }
    };

    {
        let _file_size = store_args.file_size();

        let config_root = store_args
            .root
            .map(|root| quote!(config.root = std::path::PathBuf::from(#root).into();));

        let mut args_pats = inputs
            .iter()
            .map(|input| match input {
                FnArg::Receiver(r) => abort!(r, "cachalot doesn't support struct methods"),
                FnArg::Typed(arg) => {
                    let pat = &arg.pat;

                    quote!(#pat)
                }
            })
            .collect::<Vec<_>>();

        let range_pat = args_pats.pop().unwrap();
        let key_pats = args_pats;

        quote! {
            #vis #asyncness #unsafety fn #ident <#generic_params> (#inputs) #output #where_clause {
                #use_store

                #inner_source

                let mut config = #ident::<#(#generic_type_params),*>.config::<1024>();
                #config_root

                #ident.load((#(#key_pats),*), #range_pat, &config).await
            }
        }
    }
    .into()
}
