use heck::*;
use proc_macro::TokenStream;
use wit_parser::*;

/// This only exists for testing the codegen of the Rust macro for guests where
/// it will generate a "dummy structure" which implements the `Exports` trait
/// necessary to get the generated exports bindings to compile.
#[proc_macro]
pub fn gen_dummy_export(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let input = input.trim_matches('"');
    let iface = &Interface::parse_file(&input).unwrap();
    let mut ret = quote::quote!();
    if iface.functions.len() == 0 {
        return ret.into();
    }

    let snake = quote::format_ident!("{}", iface.name.to_snake_case());
    let camel = quote::format_ident!("{}", iface.name.to_upper_camel_case());

    let mut methods = Vec::new();

    for f in iface.functions.iter() {
        let name = quote::format_ident!("{}", f.item_name().to_snake_case());
        let params = f
            .params
            .iter()
            .map(|(_, t)| quote_ty(true, iface, t))
            .collect::<Vec<_>>();
        let rets = f
            .results
            .iter_types()
            .map(|t| quote_ty(false, iface, t))
            .collect::<Vec<_>>();
        let ret = match rets.len() {
            0 => quote::quote!(()),
            1 => rets[0].clone(),
            _ => quote::quote!((#(#rets,)*)),
        };
        let method = quote::quote! {
            fn #name(#(_: #params),*) -> #ret {
                loop {}
            }
        };
        match &f.kind {
            FunctionKind::Freestanding => methods.push(method),
        }
    }
    ret.extend(quote::quote! {
        struct #camel;

        impl #snake::#camel for #camel {
            #(#methods)*
        }
    });

    ret.into()
}

fn quote_ty(param: bool, iface: &Interface, ty: &Type) -> proc_macro2::TokenStream {
    match *ty {
        Type::Bool => quote::quote! { bool },
        Type::U8 => quote::quote! { u8 },
        Type::S8 => quote::quote! { i8 },
        Type::U16 => quote::quote! { u16 },
        Type::S16 => quote::quote! { i16 },
        Type::U32 => quote::quote! { u32 },
        Type::S32 => quote::quote! { i32 },
        Type::U64 => quote::quote! { u64 },
        Type::S64 => quote::quote! { i64 },
        Type::Float32 => quote::quote! { f32 },
        Type::Float64 => quote::quote! { f64 },
        Type::Char => quote::quote! { char },
        Type::String => quote::quote! { String },
        Type::Id(id) => quote_id(param, iface, id),
    }
}

fn quote_id(param: bool, iface: &Interface, id: TypeId) -> proc_macro2::TokenStream {
    let ty = &iface.types[id];
    if let Some(name) = &ty.name {
        let name = quote::format_ident!("{}", name.to_upper_camel_case());
        let module = quote::format_ident!("{}", iface.name.to_snake_case());
        return quote::quote! { #module::#name };
    }
    match &ty.kind {
        TypeDefKind::Type(t) => quote_ty(param, iface, t),
        TypeDefKind::List(t) => {
            let t = quote_ty(param, iface, t);
            quote::quote! { Vec<#t> }
        }
        TypeDefKind::Flags(_) => panic!("unknown flags"),
        TypeDefKind::Enum(_) => panic!("unknown enum"),
        TypeDefKind::Record(_) => panic!("unknown record"),
        TypeDefKind::Variant(_) => panic!("unknown variant"),
        TypeDefKind::Union(_) => panic!("unknown union"),
        TypeDefKind::Tuple(t) => {
            let fields = t.types.iter().map(|ty| quote_ty(param, iface, ty));
            quote::quote! { (#(#fields,)*) }
        }
        TypeDefKind::Option(ty) => {
            let ty = quote_ty(param, iface, ty);
            quote::quote! { Option<#ty> }
        }
        TypeDefKind::Result(r) => {
            let ok = match &r.ok {
                Some(t) => quote_ty(param, iface, t),
                None => quote::quote!(()),
            };
            let err = match &r.err {
                Some(t) => quote_ty(param, iface, t),
                None => quote::quote!(()),
            };
            quote::quote! { Result<#ok, #err> }
        }
        TypeDefKind::Future(_) => todo!("unknown future"),
        TypeDefKind::Stream(_) => todo!("unknown stream"),
    }
}
