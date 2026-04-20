//! Unified #[module(name = "x")] expansion: CLI + RPC + events from one impl block.
//!
//! Methods with `ctx: &InvocationContext` → CLI. #[rpc_method] → RPC. #[on_event] → events.

use proc_macro::TokenStream as ProcTokenStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ImplItem, ItemImpl, PatType, Type};

use crate::cli_spec::{generate_dispatch_cli, generate_spec_code};
use crate::event_payload_map;
use crate::rpc_methods;
use std::collections::HashMap;

fn method_has_invocation_context(method: &syn::ImplItemFn) -> bool {
    for arg in method.sig.inputs.iter().skip(1) {
        if let FnArg::Typed(pt) = arg {
            if is_invocation_context_type(&pt.ty) {
                return true;
            }
        }
    }
    false
}

fn method_has_rpc_attr(method: &syn::ImplItemFn) -> bool {
    method.attrs.iter().any(|a| a.path().is_ident("rpc_method"))
}

fn method_has_on_event_attr(method: &syn::ImplItemFn) -> bool {
    method.attrs.iter().any(|a| a.path().is_ident("on_event"))
}

/// True if method has #[command] or #[cli_subcommand] — explicit marker for CLI subcommand.
fn method_has_command_attr(method: &syn::ImplItemFn) -> bool {
    method
        .attrs
        .iter()
        .any(|a| a.path().is_ident("command") || a.path().is_ident("cli_subcommand"))
}

fn is_invocation_context_type(ty: &Type) -> bool {
    if let Type::Reference(tr) = ty {
        return is_invocation_context_type(&tr.elem);
    }
    if let Type::Path(tp) = ty {
        if let Some(last) = tp.path.segments.last() {
            return last.ident == "InvocationContext";
        }
    }
    false
}

/// Filter impl to only items matching the predicate (for CLI).
///
/// CLI methods: have `ctx: &InvocationContext` and optionally `#[command]` for explicitness.
/// Methods with `#[command]` but no ctx will emit a compile error.
fn filter_impl_to_cli_methods(item: &ItemImpl) -> Result<ItemImpl, syn::Error> {
    let mut filtered = item.clone();
    let mut cli_items = Vec::new();
    for i in &item.items {
        if let ImplItem::Fn(m) = i {
            let has_ctx = method_has_invocation_context(m);
            let has_cmd = method_has_command_attr(m);
            if has_cmd && !has_ctx {
                return Err(syn::Error::new_spanned(
                    m,
                    "#[command] requires ctx: &InvocationContext in method signature",
                ));
            }
            if has_ctx && !method_has_rpc_attr(m) && !method_has_on_event_attr(m) {
                cli_items.push(i.clone());
            }
        }
    }
    filtered.items = cli_items;
    Ok(filtered)
}

/// Expand #[module(name = "demo")] on impl: add cli_spec, dispatch_cli, rpc_method_names,
/// dispatch_rpc, event_types, dispatch_event.
pub fn expand_module_impl(
    args: &syn::punctuated::Punctuated<syn::Meta, syn::token::Comma>,
    mut item: ItemImpl,
) -> ProcTokenStream {
    let module_name = crate::extract_name_from_meta(args).unwrap_or_else(|| {
        if let syn::Type::Path(tp) = &*item.self_ty {
            if let Some(seg) = tp.path.segments.last() {
                return crate::derive_module_name(&seg.ident);
            }
        }
        "module".to_string()
    });

    // 1. CLI: only from methods with InvocationContext (and not rpc/event)
    //    #[command] is optional explicit marker; if present, ctx is required
    let cli_filtered = match filter_impl_to_cli_methods(&item) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error().into(),
    };
    let has_cli = !cli_filtered.items.is_empty();
    if has_cli {
        let spec_code = generate_spec_code(&cli_filtered, &module_name);
        let cli_spec_fn: ImplItem = syn::parse2(quote! {
            /// CLI spec (from methods with ctx: &InvocationContext).
            pub fn cli_spec() -> blvm_node::module::ipc::protocol::CliSpec {
                #spec_code
            }
        })
        .expect("cli_spec fn should parse");
        item.items.push(cli_spec_fn);

        if let Some(dispatch_code) = generate_dispatch_cli(&cli_filtered) {
            let dispatch_item: ImplItem =
                syn::parse2(dispatch_code).expect("dispatch_cli should parse");
            item.items.push(dispatch_item);
        }
    } else {
        // No CLI methods - add empty cli_spec and dispatch_cli for run_module! compatibility
        let cli_spec_fn: ImplItem = syn::parse2(quote! {
            pub fn cli_spec() -> blvm_node::module::ipc::protocol::CliSpec {
                blvm_node::module::ipc::protocol::CliSpec {
                    version: 1,
                    name: #module_name.to_string(),
                    about: None,
                    subcommands: vec![],
                }
            }
        })
        .expect("cli_spec fn should parse");
        let dispatch_fn: ImplItem = syn::parse2(quote! {
            pub fn dispatch_cli(
                &self,
                _ctx: &blvm_sdk::module::runner::InvocationContext,
                subcommand: &str,
                _args: &[String],
            ) -> Result<String, blvm_node::module::traits::ModuleError> {
                Err(blvm_node::module::traits::ModuleError::Other(
                    format!("Unknown subcommand: {}", subcommand).into()
                ))
            }
        })
        .expect("dispatch_cli fn should parse");
        item.items.push(cli_spec_fn);
        item.items.push(dispatch_fn);
    }

    // 2. RPC: from #[rpc_method] methods
    item = syn::parse2(rpc_methods::expand_rpc_methods(item.clone()))
        .expect("rpc expansion should parse");

    // 3. Events: from #[on_event] methods
    let mut event_to_methods: HashMap<String, Vec<(syn::Ident, Vec<(String, bool)>, Vec<String>)>> =
        HashMap::new();
    let mut all_event_idents = Vec::new();

    for impl_item in &item.items {
        if let ImplItem::Fn(method) = impl_item {
            for attr in &method.attrs {
                if attr.path().is_ident("on_event") {
                    let event_idents = parse_on_event_args(attr);
                    let method_ident = method.sig.ident.clone();
                    let params = parse_handler_params(method);
                    let event_keys: Vec<String> =
                        event_idents.iter().map(|e| e.to_string()).collect();
                    for ev in &event_idents {
                        let key = ev.to_string();
                        if !all_event_idents
                            .iter()
                            .any(|e: &syn::Ident| e.to_string() == key)
                        {
                            all_event_idents.push(ev.clone());
                        }
                        event_to_methods.entry(key).or_default().push((
                            method_ident.clone(),
                            params.clone(),
                            event_keys.clone(),
                        ));
                    }
                    break;
                }
            }
        }
    }

    if !all_event_idents.is_empty() {
        let event_type_exprs: Vec<_> = all_event_idents
            .iter()
            .map(|i| quote! { blvm_node::module::traits::EventType::#i })
            .collect();

        let event_types_fn: ImplItem = syn::parse2(quote! {
            pub fn event_types() -> Vec<blvm_node::module::traits::EventType> {
                vec![#(#event_type_exprs),*]
            }
        })
        .expect("event_types fn should parse");

        let mut match_arms = Vec::new();
        for (ev_key, method_infos) in &event_to_methods {
            let ev_ident: syn::Ident = syn::parse_str(ev_key).unwrap();
            let payload_fields = event_payload_map::payload_fields_for_event(ev_key);

            let method_calls: Vec<proc_macro2::TokenStream> = method_infos
                .iter()
                .map(|(method_ident, params, event_types_for_method)| {
                    build_handler_call(
                        method_ident,
                        params,
                        event_types_for_method,
                        ev_key,
                        &payload_fields,
                    )
                })
                .collect();

            match_arms.push(quote! {
                blvm_node::module::traits::EventType::#ev_ident => {
                    #(#method_calls)*
                }
            });
        }
        match_arms.push(quote! { _ => {} });

        let dispatch_event_fn: ImplItem = syn::parse2(quote! {
            pub async fn dispatch_event(
                &self,
                event: blvm_node::module::ipc::protocol::EventMessage,
                ctx: &blvm_sdk::module::runner::InvocationContext,
            ) -> Result<(), blvm_node::module::traits::ModuleError> {
                use blvm_node::module::traits::EventType;
                match event.event_type {
                    #(#match_arms),*
                }
                Ok(())
            }
        })
        .expect("dispatch_event fn should parse");

        item.items.push(event_types_fn);
        item.items.push(dispatch_event_fn);
    } else {
        let event_types_fn: ImplItem = syn::parse2(quote! {
            pub fn event_types() -> Vec<blvm_node::module::traits::EventType> {
                vec![]
            }
        })
        .expect("event_types fn should parse");
        let dispatch_event_fn: ImplItem = syn::parse2(quote! {
            pub async fn dispatch_event(
                &self,
                _event: blvm_node::module::ipc::protocol::EventMessage,
                _ctx: &blvm_sdk::module::runner::InvocationContext,
            ) -> Result<(), blvm_node::module::traits::ModuleError> {
                let _ = _ctx;
                Ok(())
            }
        })
        .expect("dispatch_event fn should parse");
        item.items.push(event_types_fn);
        item.items.push(dispatch_event_fn);
    }

    // Strip #[arg] from params so emitted code compiles (extract_arg_attrs already captured the info).
    strip_arg_attrs_from_impl(&mut item);

    TokenStream::from(quote! { #item }).into()
}

/// Remove #[arg] attributes from method params. The compiler rejects macro attributes
/// on params in some contexts; we've already extracted the info for CliArgSpec.
fn strip_arg_attrs_from_impl(item: &mut ItemImpl) {
    for i in &mut item.items {
        if let ImplItem::Fn(m) = i {
            for arg in &mut m.sig.inputs {
                if let FnArg::Typed(pt) = arg {
                    strip_arg_attrs_from_pat_type(pt);
                }
            }
        }
    }
}

fn strip_arg_attrs_from_pat_type(pt: &mut PatType) {
    pt.attrs.retain(|a| !a.path().is_ident("arg"));
}

fn parse_handler_params(method: &syn::ImplItemFn) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for arg in method.sig.inputs.iter().skip(1) {
        if let syn::FnArg::Typed(pat_type) = arg {
            let name = match &*pat_type.pat {
                syn::Pat::Ident(pi) => pi.ident.to_string(),
                _ => continue,
            };
            let is_event = matches!(
                &*pat_type.ty,
                syn::Type::Reference(tr) if matches!(&*tr.elem, syn::Type::Path(tp) if tp.path.is_ident("EventMessage"))
            );
            out.push((name, is_event));
        }
    }
    out
}

fn parse_on_event_args(attr: &syn::Attribute) -> Vec<syn::Ident> {
    use syn::punctuated::Punctuated;
    use syn::token::Comma;
    let parser = Punctuated::<syn::Ident, Comma>::parse_terminated;
    attr.parse_args_with(parser)
        .map(|p| p.into_iter().collect())
        .unwrap_or_default()
}

fn is_ctx_param(name: &str) -> bool {
    matches!(name, "ctx" | "context" | "_ctx" | "_context")
}

fn build_handler_call(
    method_ident: &syn::Ident,
    params: &[(String, bool)],
    event_types_for_method: &[String],
    ev_key: &str,
    payload_fields: &Option<Vec<(&'static str, bool)>>,
) -> proc_macro2::TokenStream {
    let use_di = event_types_for_method.len() == 1
        && payload_fields.is_some()
        && params.iter().all(|(name, is_event)| {
            if *is_event {
                true
            } else if is_ctx_param(name) {
                true
            } else {
                payload_fields
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|(f, _)| f == name)
            }
        });

    if !use_di {
        let has_ctx = params.iter().any(|(name, _)| is_ctx_param(name));
        return if has_ctx {
            quote! { self.#method_ident(&event, ctx).await?; }
        } else {
            quote! { self.#method_ident(&event).await?; }
        };
    }

    let fields = payload_fields.as_ref().unwrap();
    let field_idents: Vec<syn::Ident> = fields
        .iter()
        .map(|(f, _)| syn::Ident::new(f, proc_macro2::Span::call_site()))
        .collect();
    let ev_ident = syn::Ident::new(ev_key, proc_macro2::Span::call_site());

    let call_args: Vec<proc_macro2::TokenStream> = params
        .iter()
        .map(|(name, is_event)| {
            if *is_event {
                quote! { &event }
            } else if is_ctx_param(name) {
                quote! { ctx }
            } else {
                let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                let (_, is_copy) = fields.iter().find(|(f, _)| *f == name).unwrap();
                if *is_copy {
                    quote! { *#ident }
                } else {
                    quote! { #ident }
                }
            }
        })
        .collect();

    quote! {
        if let blvm_node::module::ipc::protocol::EventPayload::#ev_ident { #(#field_idents),* } = &event.payload {
            self.#method_ident(#(#call_args),*).await?;
        }
    }
}
