use darling::FromMeta;
use proc_macro2::Ident;
use quote::format_ident;
use serde_derive_internals::attr::TagType;
use syn::{parse_macro_input, Fields};

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
            syn::FnArg::Receiver(_) => {
                todo!("idk what to do with receivers")
            }
            syn::FnArg::Typed(t) => {
                body_ty.push((*t.ty).clone());
            }
        }
    }
    let res_ty = match &fn_.sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => Some((**ty).clone()),
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
            impl ::tapi::Endpoint<AppState> for endpoint {
                fn path(&self) -> &'static str {
                    #path
                }
                fn method(&self) -> tapi::Method {
                    ::tapi::Method::#method
                }
                fn bind_to(&self, router: ::axum::Router<AppState>) -> ::axum::Router<AppState> {
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

#[derive(Debug, FromMeta)]
struct DeriveInput {
    krate: Option<String>,
}

#[proc_macro_derive(Tapi, attributes(serde, tapi))]
pub fn tapi_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(input);

    let derive_input = syn::parse2::<syn::DeriveInput>(input.clone()).unwrap();

    let tapi_path = derive_input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.meta.path().is_ident("tapi") {
                let derive_input = DeriveInput::from_meta(&attr.meta)
                    .unwrap_or_else(|_| panic!("at: {}", line!()));
                derive_input.krate.map(|krate| {
                    syn::parse_str(&krate)
                        .unwrap_or_else(|_| panic!("failed to parse krate path: {}", line!()))
                })
            } else {
                None
            }
        })
        .unwrap_or_else(|| quote::quote!(::tapi));

    let name = derive_input.ident.clone();
    let constant_name = format_ident!("{}", heck::AsShoutySnakeCase(&name.to_string()).to_string());
    let generics = derive_input.generics.params.clone();
    let mut sgenerics = Vec::new();
    let mut life_times = Vec::new();
    for g in &generics {
        match g {
            syn::GenericParam::Lifetime(l) => {
                life_times.push(l.lifetime.clone());
            }
            syn::GenericParam::Type(ty) => {
                let ident = &ty.ident;
                sgenerics.push(quote::quote!(#ident))
            }
            syn::GenericParam::Const(_) => todo!("syn::GenericParam::Const"),
        }
    }
    let serde_flags = {
        let cx = serde_derive_internals::Ctxt::new();
        let container = serde_derive_internals::ast::Container::from_ast(
            &cx,
            &derive_input,
            serde_derive_internals::Derive::Serialize,
        )
        .unwrap();
        cx.check().unwrap();
        container
    };

    if let (Some(type_into), Some(_type_from)) = (
        serde_flags.attrs.type_into(),
        serde_flags.attrs.type_try_from(),
    ) {
        return quote::quote!(
            impl<#(#life_times,)* #(#sgenerics: 'static + #tapi_path::Tapi),*> #tapi_path::Tapi for #name<#(#life_times,)* #(#sgenerics),*> {
                fn name() -> &'static str {
                    stringify!(#name)
                }
                fn id() -> std::any::TypeId {
                    std::any::TypeId::of::<#name<#(#sgenerics),*>>()
                }
                fn dependencies() -> Vec<&'static dyn #tapi_path::Typed> {
                    vec![#(<#sgenerics as #tapi_path::Tapi>::boxed()),*]
                }
                fn ts_name() -> String {
                    stringify!(#name).to_string()
                }
                fn zod_name() -> String {
                    stringify!(#name).to_string()
                }
                fn ts_decl() -> Option<String> {
                    Some(format!(
                        "export type {} = {};",
                        stringify!(#name),
                        <#type_into as #tapi_path::Tapi>::full_ts_name(),
                    ))
                }
                fn zod_decl() -> Option<String> {
                    None
                }
            }
        )
        .into();
    }

    let result: proc_macro2::TokenStream = match &derive_input.data {
        syn::Data::Struct(st) => {
            if serde_flags.attrs.transparent() {
                let inner_ty = st.fields.iter().next().unwrap().ty.clone();
                quote::quote! {
                    impl<#(#life_times,)* #(#sgenerics: 'static + #tapi_path::Tapi),*> #tapi_path::Tapi for #name<#(#life_times,)* #(#sgenerics),*> {
                        fn name() -> &'static str {
                            stringify!(#name)
                        }
                        fn id() -> std::any::TypeId {
                            std::any::TypeId::of::<#name<#(#sgenerics),*>>()
                        }
                        fn dependencies() -> Vec<&'static dyn #tapi_path::Typed> {
                            // todo!();
                            // TODO
                            vec![<#inner_ty as #tapi_path::Tapi>::boxed()]
                            // vec![#(<#fields as #tapi_path::Tapi>::boxed()),*]
                        }
                        fn ts_name() -> String {
                            stringify!(#name).to_string()
                        }
                        fn zod_name() -> String {
                            stringify!(#name).to_string()
                        }
                        fn ts_decl() -> Option<String> {
                            Some(format!(
                                "export type {} = {};",
                                stringify!(#name),
                                <#inner_ty as #tapi_path::Tapi>::full_ts_name(),
                            ))
                        }
                        fn zod_decl() -> Option<String> {
                            None
                        }
                    }
                }
            } else {
                let mut field_names = Vec::new();
                let mut fields = Vec::new();
                for field in &st.fields {
                    field_names.push(field.ident.clone().expect("field did not have a name"));
                    fields.push(field.ty.clone());
                }
                quote::quote! {
                    impl<#(#life_times,)* #(#sgenerics: 'static + #tapi_path::Tapi),*> #tapi_path::Tapi for #name<#(#life_times,)* #(#sgenerics),*> {
                        fn name() -> &'static str {
                            stringify!(#name)
                        }
                        fn id() -> std::any::TypeId {
                            std::any::TypeId::of::<#name<#(#sgenerics),*>>()
                        }
                        fn dependencies() -> Vec<&'static dyn #tapi_path::Typed> {
                            vec![#(<#fields as #tapi_path::Tapi>::boxed()),*]
                        }
                        fn ts_name() -> String {
                            stringify!(#name).to_string()
                        }
                        fn zod_name() -> String {
                            stringify!(#name).to_string()
                        }
                        fn ts_decl() -> Option<String> {
                            let fields: Vec<String> = vec![#(
                                format!(
                                    "{}: {}",
                                    stringify!(#field_names),
                                    <#fields as #tapi_path::Tapi>::full_ts_name(),
                                ),
                            )*];
                            Some(format!(
                                "export type {} = {{ {} }}",
                                stringify!(#name),
                                fields.join(", "),
                            ))
                        }
                        fn zod_decl() -> Option<String> {
                            let fields: Vec<String> = vec![#(
                                format!(
                                    "{}: {}",
                                    stringify!(#field_names),
                                    <#fields as #tapi_path::Tapi>::zod_name()
                                ),
                            )*];
                            Some(format!(
                                "const {} = z.object({{ {} }});\ntype {} = z.infer<typeof {}>;",
                                stringify!(#name),
                                fields.join(", "),
                                stringify!(#name),
                                stringify!(#name),
                            ))
                        }
                    }
                }
            }
        }
        syn::Data::Enum(en) => {
            // We want to transform an enum like this:
            // enum Analysis {
            //     Foo,
            //     Bar,
            // }
            // into something like this:
            // impl #tapi_path::Tapi for Analysis {
            //    fn name() -> &'static str {
            //        stringify!(Analysis)
            //    }
            //    fn id() -> std::any::TypeId {
            //        std::any::TypeId::of::<Analysis>()
            //    }
            //    fn dependencies() -> Vec<&'static dyn #tapi_path::Typed> {
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
            let mut deps = Vec::new();
            let generate_const = en.variants.iter().all(|v| v.fields.is_empty());
            for variant in &en.variants {
                let ident = &variant.ident;
                match (serde_flags.attrs.tag(), &variant.fields) {
                    // "X"
                    (TagType::External, Fields::Unit) => {
                        variants.push(quote::quote!(format!("{:?}", stringify!(#ident))))
                    }
                    // { "X": { "foo": string, "bar": number } }
                    (TagType::External, Fields::Named(_)) => {
                        let field_names: Vec<_> = variant
                            .fields
                            .iter()
                            .map(|f| f.ident.clone().expect("field did not have a name"))
                            .collect();
                        let fields: Vec<_> = variant.fields.iter().map(|f| f.ty.clone()).collect();
                        deps.extend_from_slice(&fields);

                        let fields = if field_names.is_empty() {
                            quote::quote!("")
                        } else {
                            quote::quote!(vec![#(
                                    format!(
                                        "{}: {}",
                                        stringify!(#field_names),
                                        <#fields as #tapi_path::Tapi>::full_ts_name(),
                                    ),
                                )*]
                            .join(", "))
                        };

                        variants.push(quote::quote!(format!(
                            "{{ {:?}: {{ {} }} }}",
                            stringify!(#ident),
                            #fields,
                        )))
                    }

                    // { "X": string }
                    // { "X": [string, number] }
                    (TagType::External, Fields::Unnamed(unnamed)) => {
                        variants.push(quote::quote!(format!(
                            "{{ {:?}: {} }}",
                            stringify!(#ident),
                            <#unnamed as #tapi_path::Tapi>::full_ts_name(),
                        )))
                    }
                    // { "tag": "X", "foo": string, "bar": number }
                    (TagType::Internal { tag }, Fields::Unit | Fields::Named(_)) => {
                        let field_names: Vec<_> = variant
                            .fields
                            .iter()
                            .map(|f| f.ident.clone().expect("field did not have a name"))
                            .collect();
                        let fields: Vec<_> = variant.fields.iter().map(|f| f.ty.clone()).collect();
                        deps.extend_from_slice(&fields);

                        let fields = if field_names.is_empty() {
                            quote::quote!("")
                        } else {
                            quote::quote!(format!(
                                ", {}",
                                vec![#(
                                    format!(
                                        "{:?}: {}",
                                        stringify!(#field_names),
                                        <#fields as #tapi_path::Tapi>::full_ts_name(),
                                    ),
                                )*]
                                .join(", "),
                            ))
                        };

                        variants.push(quote::quote!(format!(
                            "{{ {}: {:?}{} }}",
                            stringify!(#tag),
                            stringify!(#ident),
                            #fields,
                        )))
                    }
                    (TagType::Internal { tag }, Fields::Unnamed(_)) => {
                        todo!("variant: TagType::Internal, Fields::Unnamed(_)")
                    }
                    // { "tag": "X", "content": { "foo": string, "bar": number } }
                    (TagType::Adjacent { tag, content }, Fields::Unit | Fields::Named(_)) => {
                        let field_names: Vec<_> = variant
                            .fields
                            .iter()
                            .map(|f| f.ident.clone().expect("field did not have a name"))
                            .collect();
                        let fields: Vec<_> = variant.fields.iter().map(|f| f.ty.clone()).collect();
                        deps.extend_from_slice(&fields);

                        let fields = if field_names.is_empty() {
                            quote::quote!("")
                        } else {
                            quote::quote!(vec![#(
                                format!(
                                    "{:?}: {}",
                                    stringify!(#field_names),
                                    <#fields as #tapi_path::Tapi>::full_ts_name(),
                                ),
                            )*]
                            .join(", "))
                        };

                        variants.push(quote::quote!(format!(
                            "{{ {}: {:?}, {}: {{ {} }} }}",
                            stringify!(#tag),
                            stringify!(#ident),
                            stringify!(#content),
                            #fields,
                        )))
                    }
                    (TagType::Adjacent { tag, content }, Fields::Unnamed(unnamed)) => variants
                        .push(quote::quote!(format!(
                            "{{ {}: {:?}, {}: {} }}",
                            stringify!(#tag),
                            stringify!(#ident),
                            stringify!(#content),
                            <#unnamed as #tapi_path::Tapi>::full_ts_name(),
                        ))),
                    (TagType::None, Fields::Unit) => todo!("serde enum without tag"),
                    (TagType::None, Fields::Named(_)) => {
                        todo!("variant: TagType::None, Fields::Named(_)")
                    }
                    (TagType::None, Fields::Unnamed(_)) => {
                        todo!("variant: TagType::None, Fields::Unnamed(_)")
                    }
                }
            }
            quote::quote! {
                impl<#(#life_times,)* #(#sgenerics: 'static + #tapi_path::Tapi),*> #tapi_path::Tapi for #name<#(#life_times,)* #(#sgenerics),*> {
                    fn name() -> &'static str {
                        stringify!(#name)
                    }
                    fn id() -> std::any::TypeId {
                        std::any::TypeId::of::<#name>()
                    }
                    fn dependencies() -> Vec<&'static dyn #tapi_path::Typed> {
                        vec![#(<#deps as #tapi_path::Tapi>::boxed(),)*]
                    }
                    fn ts_name() -> String {
                        stringify!(#name).to_string()
                    }
                    fn zod_name() -> String {
                        stringify!(#name).to_string()
                    }
                    fn ts_decl() -> Option<String> {
                        let ty_decl = format!(
                            "export type {} = {};",
                            stringify!(#name),
                            vec![#(#variants),*].join(" | "),
                        );
                        if !#generate_const {
                            Some(ty_decl)
                        } else {
                            let data_decl = format!(
                                "export const {}: {}[] = [{}];",
                                stringify!(#constant_name),
                                stringify!(#name),
                                vec![#(#variants),*].join(", "),
                            );

                            Some(format!("{ty_decl}\n{data_decl}"))
                        }
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
        }
        syn::Data::Union(_) => todo!("unions are not supported yet"),
    };

    // let pretty = prettyplease::unparse(&syn::parse2(result.clone()).unwrap());
    // eprintln!("{pretty}");
    result.into()
}
