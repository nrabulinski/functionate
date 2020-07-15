extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
	parse_macro_input, spanned::Spanned, FnArg, Ident, ImplGenerics, ImplItem, ImplItemMethod,
	ItemImpl, ReturnType, Signature, Type, TypeGenerics, WhereClause,
};

type Generics<'a> = (ImplGenerics<'a>, TypeGenerics<'a>, Option<&'a WhereClause>);

#[proc_macro_attribute]
pub fn functionate(_args: TokenStream, item: TokenStream) -> TokenStream {
	let item = parse_macro_input!(item as ItemImpl);
	TokenStream::from(functionate_impl(item))
}

fn functionate_impl(item: ItemImpl) -> TokenStream2 {
	let ItemImpl {
		generics,
		trait_: tr,
		self_ty: ty,
		items,
		..
	} = &item;

	let ty = match ty.as_ref() {
		Type::Path(ty) => ty.path.get_ident().unwrap(),
		_ => panic!("Bad type"),
	};

	let generics = generics.split_for_impl();

	if let Some(_) = tr {
		return quote_spanned! { item.span() => compile_error!("expected bare impl"); };
	}

	let impls = items
		.iter()
		.filter_map(|item| match item {
			ImplItem::Method(method) => Some(method),
			_ => None,
		})
		.map(|method| method_to_impl(ty, &generics, method));

	quote! {
		#(#impls)*
	}
}

enum SelfTy {
	Owned,
	MutRef,
	Ref,
}

fn method_to_impl(ty: &Ident, generics: &Generics, method: &ImplItemMethod) -> TokenStream2 {
	let (impl_generics, ty_generics, where_clause) = generics;
	let recv = method.sig.receiver();
	let recv = match recv {
		Some(FnArg::Receiver(s)) => match &s.reference {
			Some(_) => match &s.mutability {
				Some(_) => SelfTy::MutRef,
				None => SelfTy::Ref,
			},
			None => SelfTy::Owned,
		},
		_ => {
			return quote_spanned! { method.sig.span() => "expected a method (custom self types are not supported)" }
		}
	};
	let Signature {
		ident,
		inputs,
		output,
		..
	} = &method.sig;
	let block = &method.block;
	let tr_ident = format_ident!("_{}Functionate_{}", ty, ident);

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

	let ret_ty = quote! { <Self as ::std::ops::FnOnce< #arg_ty >>::Output };
	let self_arg = match recv {
		SelfTy::Owned => quote! { self },
		SelfTy::MutRef => quote! { &mut self },
		SelfTy::Ref => quote! { &self },
	};

	let stream = quote! {
		#[allow(non_camel_case_types)]
		trait #tr_ident: Sized {
			type RetTy;
			fn #ident ( #inputs ) -> Self::RetTy;
		}

		impl #impl_generics #tr_ident for #ty #ty_generics #where_clause {
			type RetTy = #output;
			fn #ident ( #inputs ) -> <Self as #tr_ident >::RetTy #block
		}

		impl #impl_generics ::std::ops::FnOnce< #arg_ty > for #ty #ty_generics #where_clause {
			type Output = <Self as #tr_ident >::RetTy;
			extern "rust-call" fn call_once(mut self, #args ) -> #ret_ty {
				<Self as #tr_ident >:: #ident ( #self_arg #(, #arg_names )* )
			}
		}
	};

	if matches!(recv, SelfTy::Owned) {
		return stream;
	}

	let stream = quote! {
		#stream

		impl #impl_generics ::std::ops::FnMut< #arg_ty > for #ty #ty_generics #where_clause {
			extern "rust-call" fn call_mut(&mut self, #args ) -> #ret_ty {
				<Self as #tr_ident >:: #ident (self #(, #arg_names )* )
			}
		}
	};

	if matches!(recv, SelfTy::MutRef) {
		return stream;
	}

	quote! {
		#stream

		impl #impl_generics ::std::ops::Fn< #arg_ty > for #ty #ty_generics #where_clause {
			extern "rust-call" fn call(&self, #args ) -> #ret_ty {
				<Self as #tr_ident >:: #ident (self #(, #arg_names )* )
			}
		}
	}
}
