//! Generate rpc_method_names() and dispatch_rpc() from #[rpc_method] methods.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItem, ItemImpl, Meta};

/// Extract rpc method name from #[rpc_method(name = "X")] attribute.
/// Returns None when no explicit name is given (caller should use method ident).
fn extract_rpc_method_name(attr: &syn::Attribute) -> Option<String> {
    if !attr.path().is_ident("rpc_method") {
        return None;
    }
    let meta = attr.parse_args::<Meta>().ok()?;
    if let Meta::NameValue(nv) = meta {
        if nv.path.is_ident("name") {
            if let syn::Expr::Lit(el) = &nv.value {
                if let syn::Lit::Str(s) = &el.lit {
                    return Some(s.value());
                }
            }
        }
    }
    None
}

/// Check if method has #[rpc_method] (with or without name).
fn has_rpc_method_attr(method: &syn::ImplItemFn) -> bool {
    method.attrs.iter().any(|a| a.path().is_ident("rpc_method"))
}

/// Generate rpc_method_names() and dispatch_rpc() for impl block with #[rpc_method] methods.
pub fn expand_rpc_methods(mut item: ItemImpl) -> TokenStream {
    let mut rpc_methods: Vec<(syn::Ident, String)> = Vec::new();

    for impl_item in &item.items {
        if let ImplItem::Fn(method) = impl_item {
            if !has_rpc_method_attr(method) {
                continue;
            }
            let name = method
                .attrs
                .iter()
                .find_map(extract_rpc_method_name)
                .unwrap_or_else(|| method.sig.ident.to_string());
            rpc_methods.push((method.sig.ident.clone(), name));
        }
    }

    if rpc_methods.is_empty() {
        let empty_rpc_names: ImplItem = syn::parse2(quote! {
            pub fn rpc_method_names() -> Vec<&'static str> {
                vec![]
            }
        })
        .expect("empty rpc_method_names fn should parse");
        let dispatch_rpc_stub: ImplItem = syn::parse2(quote! {
            pub fn dispatch_rpc(
                &self,
                method: &str,
                _params: &serde_json::Value,
                _db: &std::sync::Arc<dyn blvm_node::storage::database::Database>,
            ) -> Result<serde_json::Value, blvm_node::module::traits::ModuleError> {
                Err(blvm_node::module::traits::ModuleError::Other(
                    format!("Unknown RPC method: {}", method).into()
                ))
            }
        })
        .expect("dispatch_rpc stub should parse");
        item.items.push(empty_rpc_names);
        item.items.push(dispatch_rpc_stub);
        return quote! { #item };
    }

    let names: Vec<String> = rpc_methods.iter().map(|(_, n)| n.clone()).collect();
    let name_lits: Vec<TokenStream> = names
        .iter()
        .map(|n| {
            let lit = proc_macro2::Literal::string(n);
            quote! { #lit }
        })
        .collect();

    let rpc_method_names_fn: ImplItem = syn::parse2(quote! {
        /// RPC method names for registration (from #[rpc_method] handlers).
        pub fn rpc_method_names() -> Vec<&'static str> {
            vec![#(#name_lits),*]
        }
    })
    .expect("rpc_method_names fn should parse");

    let mut match_arms = Vec::new();
    for (method_ident, name) in &rpc_methods {
        let name_lit = proc_macro2::Literal::string(name);
        match_arms.push(quote! {
            #name_lit => self.#method_ident(params, db),
        });
    }

    let dispatch_rpc_fn: ImplItem = syn::parse2(quote! {
        /// Dispatch RPC invocation to the matching #[rpc_method] handler.
        pub fn dispatch_rpc(
            &self,
            method: &str,
            params: &serde_json::Value,
            db: &std::sync::Arc<dyn blvm_node::storage::database::Database>,
        ) -> Result<serde_json::Value, blvm_node::module::traits::ModuleError> {
            match method {
                #(#match_arms)*
                _ => Err(blvm_node::module::traits::ModuleError::Other(
                    format!("Unknown RPC method: {}", method).into()
                )),
            }
        }
    })
    .expect("dispatch_rpc fn should parse");

    item.items.push(rpc_method_names_fn);
    item.items.push(dispatch_rpc_fn);

    quote! { #item }
}
