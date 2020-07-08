#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(proc_macro_quote)]
#![feature(proc_macro_hygiene)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, FnArg, ImplGenerics, ImplItem, ImplItemMethod, ItemImpl, ReturnType,
    Signature, Type, TypeGenerics, WhereClause,
};

type Generics<'a> = (ImplGenerics<'a>, TypeGenerics<'a>, Option<&'a WhereClause>);

#[proc_macro_attribute]
pub fn functionate(_args: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemImpl);

    let ItemImpl {
        generics,
        trait_: tr,
        self_ty: ty,
        items,
        ..
    } = &item;

    let generics = generics.split_for_impl();

    if let Some(_) = tr {
        //quote_spanned! { item.span() => compile_error!("expected bare impl"); };
        panic!("expected bare impl");
    }

    let impls = items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Method(method) => Some(method),
            _ => None,
        })
        .map(|method| method_to_impl(ty, &generics, method));

    TokenStream::from(quote! {
        #item

        #(#impls)*
    })
}

enum SelfTy {
    Owned,
    MutRef,
    Ref,
}

fn method_to_impl(ty: &Type, generics: &Generics, method: &ImplItemMethod) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics;
    let recv = method.sig.receiver();
    let recv = match recv {
        None | Some(FnArg::Typed(_)) => {
            panic!("expected a method (custom self types are not supported)")
        }
        Some(FnArg::Receiver(s)) => match &s.reference {
            Some(_) => match &s.mutability {
                Some(_) => SelfTy::MutRef,
                None => SelfTy::Ref,
            },
            None => SelfTy::Owned,
        },
    };
    let Signature {
        ident,
        inputs,
        output,
        ..
    } = &method.sig;

    let output = match output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => ty.into_token_stream(),
    };

    let arg_names = inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(ty) => Some(ty.pat.to_token_stream()),
        })
        .collect::<Vec<_>>();

    let arg_ty = inputs.iter().filter_map(|arg| match arg {
        FnArg::Receiver(_) => None,
        FnArg::Typed(ty) => Some(ty.ty.to_token_stream()),
    });
    let arg_ty = quote! { ( #(#arg_ty ,)* ) };

    let args = if arg_names.is_empty() {
        quote! { _: () }
    } else {
        quote! { ( #(#arg_names,)* ): #arg_ty }
    };

    let stream = quote! {
        impl #impl_generics ::std::ops::FnOnce< #arg_ty > for #ty #ty_generics #where_clause {
            type Output = #output;
            extern "rust-call" fn call_once(mut self, #args ) -> #output {
                self. #ident ( #(#arg_names),* )
            }
        }
    };

    if matches!(recv, SelfTy::Owned) {
        return stream;
    }

    let stream = quote! {
        #stream

        impl #impl_generics ::std::ops::FnMut< #arg_ty > for #ty #ty_generics #where_clause {
            extern "rust-call" fn call_mut(&mut self, #args ) -> #output {
                self. #ident ( #(#arg_names),* )
            }
        }
    };

    if matches!(recv, SelfTy::MutRef) {
        return stream;
    }

    quote! {
        #stream

        impl #impl_generics ::std::ops::Fn< #arg_ty > for #ty #ty_generics #where_clause {
            extern "rust-call" fn call(&self, #args ) -> #output {
                self. #ident ( #(#arg_names),* )
            }
        }
    }
}
