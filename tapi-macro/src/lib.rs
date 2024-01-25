use proc_macro2::Ident;
use quote::format_ident;
use syn::parse_macro_input;

#[derive(Debug)]
struct Args {
    path: String,
    method: Ident,
}

impl syn::parse::Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated(input).map(
            |punctuated| {
                let mut path = None;
                let mut method = None;
                for meta in punctuated {
                    match meta {
                        syn::Meta::NameValue(syn::MetaNameValue {
                            path: syn::Path { segments, .. },
                            value,
                            ..
                        }) => {
                            let ident = segments.first().unwrap().ident.to_string();
                            match ident.as_str() {
                                "path" => {
                                    path = {
                                        match value {
                                            syn::Expr::Lit(syn::ExprLit {
                                                lit: syn::Lit::Str(lit_str),
                                                ..
                                            }) => Some(lit_str.value()),
                                            _ => panic!("unknown attribute"),
                                        }
                                    }
                                }
                                "method" => {
                                    method = {
                                        match value {
                                            syn::Expr::Path(syn::ExprPath { path, .. }) => {
                                                Some(path.segments.first().unwrap().ident.clone())
                                            }
                                            _ => panic!("unknown attribute"),
                                        }
                                    }
                                }
                                _ => panic!("unknown attribute"),
                            }
                        }
                        _ => panic!("unknown attribute"),
                    }
                }
                Args {
                    path: path.unwrap(),
                    method: method.unwrap(),
                }
            },
        )
    }
}

#[proc_macro_attribute]
pub fn tapi(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = proc_macro2::TokenStream::from(item);

    let Args { path, method } = parse_macro_input!(attr as Args);

    let fn_ = syn::parse2::<syn::ItemFn>(item.clone()).unwrap();

    let name = fn_.sig.ident;
    let mut body_ty = Vec::new();
    for inp in &fn_.sig.inputs {
        match inp {
            syn::FnArg::Receiver(r) => {
                todo!("idk what to do with receivers")
            }
            syn::FnArg::Typed(t) => {
                body_ty.push((&*t.ty).clone());
            }
        }
    }
    let res_ty = match &fn_.sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => Some((&**ty).clone()),
    };

    let res_ty = res_ty.unwrap_or_else(|| {
        syn::parse2::<syn::Type>(quote::quote! {
            ()
        })
        .unwrap()
    });

    let handler = match method.to_string().as_str() {
        "Get" => format_ident!("get"),
        "Post" => format_ident!("post"),
        "Put" => format_ident!("put"),
        "Delete" => format_ident!("delete"),
        "Patch" => format_ident!("patch"),
        _ => todo!("unknown method: {}", method.to_string()),
    };

    let output = quote::quote! {
        mod #name {
            use super::*;
            pub struct endpoint;
            impl ::tapi::Endpoint for endpoint {
                fn path(&self) -> &'static str {
                    #path
                }
                fn method(&self) -> tapi::Method {
                    ::tapi::Method::#method
                }
                fn bind_to(&self, router: ::axum::Router) -> ::axum::Router {
                    router.route(#path, ::axum::routing::#handler(super::#name))
                }
                fn body(&self) -> ::tapi::RequestStructure {
                    let mut s = ::tapi::RequestStructure::new(::tapi::Method::#method);
                    #(
                        s.merge_with(
                            <#body_ty as ::tapi::RequestTapiExtractor>::extract_request()
                        );
                    )*
                    s
                }
                fn res(&self) -> ::tapi::ResponseTapi {
                    <#res_ty as ::tapi::ResponseTapiExtractor>::extract_response()
                }
            }
        }

        #item
    };
    output.into()
}

#[proc_macro_derive(Tapi)]
pub fn tapi_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(input);

    let derive_input = syn::parse2::<syn::DeriveInput>(input.clone()).unwrap();

    let name = derive_input.ident;

    match &derive_input.data {
        syn::Data::Struct(st) => {
            let mut field_names = Vec::new();
            let mut fields = Vec::new();
            for field in &st.fields {
                field_names.push(field.ident.clone().unwrap());
                fields.push(field.ty.clone());
            }
            quote::quote! {
                impl ::tapi::Tapi for #name {
                    fn name() -> &'static str {
                        stringify!(#name)
                    }
                    fn id() -> std::any::TypeId {
                        std::any::TypeId::of::<#name>()
                    }
                    fn dependencies() -> Vec<&'static dyn ::tapi::Typed> {
                        vec![#(<#fields as ::tapi::Tapi>::boxed()),*]
                    }
                    fn ts_name() -> String {
                        stringify!(#name).to_string()
                    }
                    fn zod_name() -> String {
                        stringify!(#name).to_string()
                    }
                    fn ts_decl() -> Option<String> {
                        Some(format!(
                            "export type {} = {{ {} }}",
                            stringify!(#name),
                            vec![#(
                                format!(
                                    "{}: {}",
                                    stringify!(#field_names),
                                    <#fields as ::tapi::Tapi>::ts_name()
                                ),
                            )*].join(", ")
                        ))
                    }
                    fn zod_decl() -> Option<String> {
                        Some(format!(
                            "const {} = z.object({{ {} }});\ntype {} = z.infer<typeof {}>;",
                            stringify!(#name),
                            vec![#(
                                format!(
                                    "{}: {}",
                                    stringify!(#field_names),
                                    <#fields as ::tapi::Tapi>::zod_name()
                                ),
                            )*].join(", "),
                            stringify!(#name),
                            stringify!(#name),
                        ))
                    }
                }
            }
            .into()
        }
        syn::Data::Enum(en) => {
            // We want to transform an enum like this:
            // enum Analysis {
            //     Foo,
            //     Bar,
            // }
            // into something like this:
            // impl ::tapi::Tapi for Analysis {
            //    fn name() -> &'static str {
            //        stringify!(Analysis)
            //    }
            //    fn id() -> std::any::TypeId {
            //        std::any::TypeId::of::<Analysis>()
            //    }
            //    fn dependencies() -> Vec<&'static dyn ::tapi::Typed> {
            //        Vec::new()
            //    }
            //    fn ts_name() -> String {
            //        stringify!(Analysis).to_string()
            //    }
            //    fn zod_name() -> String {
            //        stringify!(Analysis).to_string()
            //    }
            //    fn ts_decl() -> Option<String> {
            //        Some(format!(
            //            "export type {} = {};\nexport const {}: {}[] = [{}];",
            //            stringify!(Analysis),
            //            vec!["Foo", "Bar"].join(" | ")",
            //            stringify!(Analysis),
            //            stringify!(Analysis).to_uppercase(),
            //            vec!["Foo", "Bar"].join(" | ")",
            //        ))
            //    }
            //    fn zod_decl() -> Option<String> {
            //        Some(format!(
            //            "const {} = z.union([z.literal('Foo'), z.literal('Bar')]);\ntype {} = z.infer<typeof {}>;",
            //            stringify!(Analysis),
            //        ))
            //    }
            // }
            let mut variants = Vec::new();
            for variant in &en.variants {
                let ident = &variant.ident;
                variants.push(ident);
                // variants.push(quote::quote! {
                //     stringify!(#ident),
                // });
            }
            quote::quote! {
                impl ::tapi::Tapi for #name {
                    fn name() -> &'static str {
                        stringify!(#name)
                    }
                    fn id() -> std::any::TypeId {
                        std::any::TypeId::of::<#name>()
                    }
                    fn dependencies() -> Vec<&'static dyn ::tapi::Typed> {
                        Vec::new()
                    }
                    fn ts_name() -> String {
                        stringify!(#name).to_string()
                    }
                    fn zod_name() -> String {
                        stringify!(#name).to_string()
                    }
                    fn ts_decl() -> Option<String> {
                        Some(format!(
                            "export type {} = {};\nexport const {}: {}[] = [{}];",
                            stringify!(#name),
                            vec![#(format!("{:?}", stringify!(#variants))),*].join(" | "),
                            stringify!(#name).to_uppercase(),
                            stringify!(#name),
                            vec![#(format!("{:?}", stringify!(#variants))),*].join(", "),
                        ))
                    }
                    fn zod_decl() -> Option<String> {
                        Some(format!(
                            "const {} = z.union([{}]);\ntype {} = z.infer<typeof {}>;",
                            stringify!(#name),
                            vec![#(stringify!(#variants)),*].join(", "),
                            stringify!(#name),
                            stringify!(#name),
                        ))
                    }
                }
            }
            .into()
        }
        syn::Data::Union(_) => todo!(),
    }
}
