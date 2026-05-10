//! Procedural macros for BLVM SDK CLI module system.

mod cli_spec;
mod event_payload_map;
mod module_impl;
mod rpc_methods;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, DeriveInput, Field, ImplItem, Item,
    ItemImpl, LitStr, Meta,
};

type EventToMethodsMap = HashMap<String, Vec<(syn::Ident, Vec<(String, bool)>, Vec<String>)>>;

/// Attribute macro: `#[command(name = "...")]` or `#[module_cli(name = "...")]` - on struct: marks CLI entry point.
/// On impl block: generates `cli_spec()`. Same attribute works for both.
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    match &item {
        Item::Struct(_) => expand_module_cli(attr, item),
        Item::Impl(impl_item) => expand_cli_subcommand(attr, impl_item.clone()),
        _ => TokenStream::from(quote! { #item }),
    }
}

/// Attribute macro: `#[module_cli(name = "...")]` - marks a struct as the module CLI entry point.
///
/// The `name` is the CLI command name (e.g. "sync-policy"). Emits `CLI_NAME` const.
#[proc_macro_attribute]
pub fn module_cli(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    expand_module_cli(attr, item)
}

fn expand_module_cli(attr: TokenStream, item: Item) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Comma>::parse_terminated);
    let name = extract_name_from_meta(&args).unwrap_or_else(|| "cli".to_string());

    let (struct_name, item_tokens) = match &item {
        Item::Struct(s) => (s.ident.clone(), quote! { #item }),
        _ => return TokenStream::from(quote! { #item }),
    };

    let name_lit = LitStr::new(&name, proc_macro2::Span::call_site());
    let impl_block = quote! {
        impl #struct_name {
            /// CLI command name (from #[command(name = "...")]).
            pub const CLI_NAME: &str = #name_lit;
        }
    };

    TokenStream::from(quote! {
        #item_tokens
        #impl_block
    })
}

/// Attribute macro: `#[cli_subcommand]` or `#[cli_subcommand(name = "...")]` - on impl block.
///
/// Generates `cli_spec()` function returning `blvm_node::module::ipc::protocol::CliSpec`.
#[proc_macro_attribute]
pub fn cli_subcommand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemImpl);
    expand_cli_subcommand(attr, item)
}

fn expand_cli_subcommand(attr: TokenStream, mut item: ItemImpl) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Comma>::parse_terminated);
    let cli_name = extract_name_from_meta(&args);

    let derived_name = match &item.self_ty.as_ref() {
        syn::Type::Path(p) => {
            let ty_name = p
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            if ty_name.ends_with("Cli") {
                let base = &ty_name[..ty_name.len() - 3];
                base.chars()
                    .enumerate()
                    .flat_map(|(i, c)| {
                        if c == '_' {
                            vec!['-']
                        } else if c.is_uppercase() && i > 0 {
                            vec!['-', c.to_lowercase().next().unwrap()]
                        } else if c.is_uppercase() {
                            vec![c.to_lowercase().next().unwrap()]
                        } else {
                            vec![c]
                        }
                    })
                    .collect::<String>()
            } else {
                ty_name.to_lowercase().replace('_', "-")
            }
        }
        _ => "cli".to_string(),
    };

    let cli_name = cli_name.unwrap_or(derived_name);

    let spec_code = cli_spec::generate_spec_code(&item, &cli_name);

    let fn_item: ImplItem = syn::parse2(quote! {
        /// Generated CLI spec for module registration.
        pub fn cli_spec() -> blvm_node::module::ipc::protocol::CliSpec {
            #spec_code
        }
    })
    .expect("generated fn should parse");

    item.items.push(fn_item);

    if let Some(dispatch_code) = cli_spec::generate_dispatch_cli(&item) {
        let dispatch_item: ImplItem =
            syn::parse2(dispatch_code).expect("dispatch_cli should parse");
        item.items.push(dispatch_item);
    }

    TokenStream::from(quote! { #item })
}

pub(crate) fn extract_name_from_meta(args: &Punctuated<Meta, Comma>) -> Option<String> {
    for meta in args {
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("name") {
                if let syn::Expr::Lit(el) = &nv.value {
                    if let syn::Lit::Str(s) = &el.lit {
                        return Some(s.value());
                    }
                }
            }
        }
    }
    None
}

/// Extract config type from #[module(config = DemoConfig)].
pub(crate) fn extract_config_type_from_meta(args: &Punctuated<Meta, Comma>) -> Option<syn::Type> {
    for meta in args {
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("config") {
                return syn::parse2(nv.value.to_token_stream()).ok();
            }
        }
    }
    None
}

/// Infer config type from struct field `config: T`.
fn extract_config_type_from_struct(struct_item: &syn::ItemStruct) -> Option<syn::Type> {
    if let syn::Fields::Named(ref fields) = struct_item.fields {
        for f in &fields.named {
            if f.ident.as_ref().map(|i| i == "config").unwrap_or(false) {
                return Some(f.ty.clone());
            }
        }
    }
    None
}

/// Derive module name from struct/type name: DemoModule → "demo", SyncPolicy → "sync-policy".
pub(crate) fn derive_module_name(ident: &syn::Ident) -> String {
    let s = ident.to_string();
    let s = s.strip_suffix("Module").unwrap_or(&s);
    s.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if c == '_' {
                vec!['-']
            } else if c.is_uppercase() && i > 0 {
                vec!['-', c.to_lowercase().next().unwrap()]
            } else if c.is_uppercase() {
                vec![c.to_lowercase().next().unwrap()]
            } else {
                vec![c]
            }
        })
        .collect::<String>()
}

/// Extract migrations from #[module(migrations = ((1, up_initial), (2, up_add_items_tree)))].
/// Returns Vec of (version_lit, fn_ident) for generating the migrations slice.
fn extract_migrations_from_meta(args: &Punctuated<Meta, Comma>) -> Option<Vec<(u32, syn::Ident)>> {
    for meta in args {
        if let Meta::NameValue(nv) = meta {
            if !nv.path.is_ident("migrations") {
                continue;
            }
            let expr = &nv.value;
            let mut pairs = Vec::new();
            if let syn::Expr::Tuple(outer) = expr {
                for elem in &outer.elems {
                    if let syn::Expr::Tuple(inner) = elem {
                        let elems: Vec<_> = inner.elems.iter().collect();
                        if elems.len() >= 2 {
                            let version = match &elems[0] {
                                syn::Expr::Lit(el) => {
                                    if let syn::Lit::Int(li) = &el.lit {
                                        li.base10_parse::<u32>().ok()
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            };
                            let ident = match &elems[1] {
                                syn::Expr::Path(ep) => ep.path.get_ident().cloned(),
                                _ => None,
                            };
                            if let (Some(v), Some(i)) = (version, ident) {
                                pairs.push((v, i));
                            }
                        }
                    }
                }
            }
            if !pairs.is_empty() {
                return Some(pairs);
            }
        }
    }
    None
}

/// Attribute macro: `#[rpc_methods]` - on impl block with #[rpc_method] methods.
///
/// Generates `rpc_method_names() -> Vec<&'static str>` and `dispatch_rpc(&self, method, params, db)`.
/// Enables auto-discovery of RPC methods for run_module!.
#[proc_macro_attribute]
pub fn rpc_methods(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemImpl);
    rpc_methods::expand_rpc_methods(item).into()
}

/// Attribute macro: `#[rpc_method]` or `#[rpc_method(name = "...")]`.
///
/// Marks a method as an RPC endpoint. Without `name`, the function name is used (e.g. `demo_set` → "demo_set").
/// Use with #[rpc_methods] or #[module(name = "x")] on the impl block for auto-discovery.
#[proc_macro_attribute]
pub fn rpc_method(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ImplItem);
    TokenStream::from(quote! { #item })
}

/// Attribute macro: `#[migration(version = N)]` or `#[migration(version = N, down)]`.
///
/// Marks a function as a migration step. For `up` migrations (default), the function
/// must have signature `fn(&MigrationContext) -> Result<()>`. Use with `run_migrations`:
///
/// ```ignore
/// run_migrations(&db, &[(1, up_initial), (2, up_add_cache)])?;
/// ```
///
/// `down` migrations are for future rollback support; currently pass-through only.
#[proc_macro_attribute]
pub fn migration(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Comma>::parse_terminated);
    let item = parse_macro_input!(item as syn::ItemFn);

    let mut version = None;
    let mut is_down = false;

    for meta in args {
        match meta {
            Meta::NameValue(nv) if nv.path.is_ident("version") => {
                if let syn::Expr::Lit(el) = &nv.value {
                    if let syn::Lit::Int(li) = &el.lit {
                        version = li.base10_parse::<u32>().ok();
                    }
                }
            }
            Meta::Path(p) if p.is_ident("down") => is_down = true,
            _ => {}
        }
    }

    if version.is_none() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[migration] requires version = N (e.g. #[migration(version = 1)])",
        )
        .to_compile_error()
        .into();
    }

    let _version = version.unwrap();
    let _is_down = is_down;
    // Pass through; module author collects migrations and passes to run_migrations
    TokenStream::from(quote! { #item })
}

/// Extract env var name from `#[config_env]` or `#[config_env("ENV_NAME")]` on a field.
/// Returns None if no config_env attr; Some(env_name) if present (None = use default).
fn extract_config_env_from_field(field: &Field) -> Option<Option<String>> {
    for attr in &field.attrs {
        if attr.path().is_ident("config_env") {
            // #[config_env] with no args is Meta::Path; parse_args fails on empty.
            // #[config_env("X")] is Meta::List. Use meta structure directly.
            return Some(match &attr.meta {
                Meta::Path(_) => None, // #[config_env] - use default MODULE_CONFIG_<FIELD>
                Meta::NameValue(nv) if nv.path.is_ident("env") => {
                    if let syn::Expr::Lit(el) = &nv.value {
                        if let syn::Lit::Str(s) = &el.lit {
                            Some(s.value())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Meta::List(list) => syn::parse2::<LitStr>(list.tokens.clone())
                    .ok()
                    .map(|s| s.value()),
                _ => None,
            });
        }
    }
    None
}

/// Generate env override assignment for a field based on its type.
fn env_override_stmt(
    field_name: &syn::Ident,
    env_lit: &LitStr,
    ty: &syn::Type,
) -> proc_macro2::TokenStream {
    let ty_str = ty.to_token_stream().to_string();
    let ty_compact = ty_str.replace(' ', "");
    if ty_compact == "String" {
        quote! {
            if let Ok(__v) = std::env::var(#env_lit) {
                self.#field_name = __v;
            }
        }
    } else if ty_compact.starts_with("Option<") {
        quote! {
            if let Ok(__v) = std::env::var(#env_lit) {
                self.#field_name = Some(__v);
            }
        }
    } else if ty_compact.starts_with("Vec<") {
        quote! {
            if let Ok(__v) = std::env::var(#env_lit) {
                self.#field_name = __v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
        }
    } else if ty_compact == "bool" {
        quote! {
            if let Ok(__v) = std::env::var(#env_lit) {
                self.#field_name = __v.eq_ignore_ascii_case("true") || __v == "1";
            }
        }
    } else {
        quote! {
            if let Ok(__v) = std::env::var(#env_lit) {
                if let Ok(__parsed) = __v.parse() {
                    self.#field_name = __parsed;
                }
            }
        }
    }
}

/// Attribute macro: `#[config(name = "...")]` or `#[module_config(name = "...")]` - marks a config struct.
///
/// The `name` matches the node config section `[modules.<name>]` for override merging.
/// Emits `CONFIG_SECTION_NAME` const. Config loading (from env vars or file) is module-specific.
///
/// Field-level `#[config_env]` or `#[config_env("ENV_NAME")]`: env override for that field.
/// - `#[config_env]` → uses `MODULE_CONFIG_<FIELD_UPPERCASE>` (node passes these)
/// - `#[config_env("CUSTOM_ENV")]` → uses `CUSTOM_ENV` for standalone/override
///
/// Generates `apply_env_overrides(&mut self)` to apply env vars over loaded config.
#[proc_macro_attribute]
pub fn config(attr: TokenStream, item: TokenStream) -> TokenStream {
    module_config(attr, item)
}

/// Strip #[config_env] from struct fields so derive macros (Serialize, etc.) don't see it.
fn strip_config_env_from_item(item: &Item) -> Item {
    let Item::Struct(mut s) = item.clone() else {
        return item.clone();
    };
    if let syn::Fields::Named(ref mut fields) = s.fields {
        for field in &mut fields.named {
            field.attrs.retain(|a| !a.path().is_ident("config_env"));
        }
    }
    Item::Struct(s)
}

#[proc_macro_attribute]
pub fn module_config(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Comma>::parse_terminated);
    let item = parse_macro_input!(item as Item);

    let name = extract_name_from_meta(&args);

    // Output struct without #[config_env] so derive macros don't error on it
    let item_stripped = strip_config_env_from_item(&item);
    let item_tokens = quote! { #item_stripped };

    let extra = if let Some(config_name) = name {
        let name_lit = LitStr::new(&config_name, proc_macro2::Span::call_site());
        if let Item::Struct(s) = &item {
            let struct_name = &s.ident;
            let mut apply_stmts = Vec::new();
            if let syn::Fields::Named(fields) = &s.fields {
                for field in &fields.named {
                    let field_name = field.ident.as_ref().expect("named field");
                    let Some(env_opt) = extract_config_env_from_field(field) else {
                        continue;
                    };
                    let env_var = env_opt.unwrap_or_else(|| {
                        format!(
                            "MODULE_CONFIG_{}",
                            field_name.to_string().to_uppercase().replace('-', "_")
                        )
                    });
                    let env_lit = LitStr::new(&env_var, proc_macro2::Span::call_site());
                    let ty = &field.ty;
                    let set_stmt = env_override_stmt(field_name, &env_lit, ty);
                    apply_stmts.push(set_stmt);
                }
            }
            let apply_block = if apply_stmts.is_empty() {
                quote! {
                    /// Apply env overrides to config. No #[config_env] fields; no-op.
                    pub fn apply_env_overrides(&mut self) {}
                }
            } else {
                quote! {
                    /// Apply env overrides for fields marked with #[config_env].
                    /// Call after loading from file; env vars override file values.
                    pub fn apply_env_overrides(&mut self) {
                        #(#apply_stmts)*
                    }
                }
            };

            let load_block = quote! {
                /// Load config from path (e.g. config.toml), apply env overrides.
                /// Requires `#[derive(Default, Serialize, Deserialize)]` on the struct.
                pub fn load(path: impl std::convert::AsRef<std::path::Path>) -> std::result::Result<Self, anyhow::Error> {
                    let mut config: Self = std::fs::read_to_string(path.as_ref())
                        .ok()
                        .and_then(|s| toml::from_str(&s).ok())
                        .unwrap_or_else(|| Self::default());
                    config.apply_env_overrides();
                    Ok(config)
                }
            };

            quote! {
                impl #struct_name {
                    /// Config section name (from #[module_config(name = "...")]).
                    /// Matches [modules.<name>] in node config for override merging.
                    pub const CONFIG_SECTION_NAME: &str = #name_lit;
                    #apply_block
                    #load_block
                }
            }
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };

    TokenStream::from(quote! {
        #item_tokens
        #extra
    })
}

/// Attribute macro: `#[module]` or `#[blvm_module]` - marks the main module struct.
/// With `#[module(name = "demo")]` on an impl block: generates CLI, RPC, and event
/// dispatch from a single impl (replaces #[command], #[rpc_methods], #[event_handlers]).
#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    blvm_module(attr, item)
}

/// Attribute macro: `#[blvm_module]` - marks the main module struct.
/// On struct: `#[module(name = "demo", config = DemoConfig)]` generates `__module_new(config)`.
/// With `migrations = ((1, up_initial), (2, up_add_items_tree))` also generates `ModuleMeta` impl
/// for `run_module_main!(DemoModule)`.
/// On impl: `#[module(name = "demo")]` generates cli_spec, dispatch_cli, rpc_method_names, etc.
#[proc_macro_attribute]
pub fn blvm_module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    let args = parse_macro_input!(attr with Punctuated::<Meta, Comma>::parse_terminated);
    match &item {
        Item::Struct(struct_item) => {
            let struct_name = &struct_item.ident;
            let config_ty = extract_config_type_from_meta(&args)
                .or_else(|| extract_config_type_from_struct(struct_item));
            let migrations = extract_migrations_from_meta(&args);
            let module_name =
                extract_name_from_meta(&args).unwrap_or_else(|| derive_module_name(struct_name));

            let mut blocks = Vec::new();
            let ct = config_ty.as_ref();

            if let (Some(ct), Some(migs)) = (ct, migrations) {
                let name_lit = LitStr::new(&module_name, proc_macro2::Span::call_site());
                let migration_entries: Vec<_> = migs
                    .iter()
                    .map(|(v, ident)| quote! { (#v, #ident as blvm_sdk::module::MigrationUp) })
                    .collect();
                let meta_impl = quote! {
                    impl blvm_sdk::module::ModuleMeta for #struct_name {
                        const MODULE_NAME: &'static str = #name_lit;
                        type Config = #ct;
                        fn migrations() -> &'static [(u32, blvm_sdk::module::MigrationUp)] {
                            static MIGRATIONS: &[(u32, blvm_sdk::module::MigrationUp)] = &[#(#migration_entries),*];
                            MIGRATIONS
                        }
                        fn __module_new(config: Self::Config) -> Self {
                            Self { config }
                        }
                    }
                };
                blocks.push(meta_impl);
            } else if let Some(ct) = ct {
                let impl_block = quote! {
                    impl #struct_name {
                        #[doc(hidden)]
                        pub fn __module_new(config: #ct) -> Self {
                            Self { config }
                        }
                    }
                };
                blocks.push(impl_block);
            }

            if blocks.is_empty() {
                TokenStream::from(quote! { #item })
            } else {
                TokenStream::from(quote! {
                    #item
                    #(#blocks)*
                })
            }
        }
        Item::Impl(impl_item) => module_impl::expand_module_impl(&args, impl_item.clone()),
        _ => TokenStream::from(quote! { #item }),
    }
}

/// Attribute macro: `#[on_event(NewBlock, NewTransaction, ...)]` - marks a method as event handler.
/// Use with `#[event_handlers]` on the impl block. Pass-through; event_handlers reads this.
#[proc_macro_attribute]
pub fn on_event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Attribute macro: `#[event_handlers]` - on impl block with #[on_event] methods.
///
/// Generates:
/// - `event_types() -> Vec<EventType>` — all event types from #[on_event] handlers (for subscribe_events)
/// - `dispatch_event(&self, event: EventMessage) -> impl Future` — dispatches to the right handler
///
/// Auto-subscribe: call `integration.subscribe_events(MyModule::event_types()).await?` after connect.
/// Auto-unsubscribe: happens when module unloads (IPC connection closes).
#[proc_macro_attribute]
pub fn event_handlers(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl_block = parse_macro_input!(item as ItemImpl);

    // Collect (event_type_ident, (method_ident, params, event_types_for_method)) from each #[on_event] method
    let mut event_to_methods: EventToMethodsMap = HashMap::new();
    let mut all_event_idents = Vec::<syn::Ident>::new();

    for impl_item in &impl_block.items {
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
                        if !all_event_idents.iter().any(|e| e == ev) {
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

    if all_event_idents.is_empty() {
        return TokenStream::from(quote! { #impl_block });
    }

    // Generate event_types()
    let event_type_exprs: Vec<_> = all_event_idents
        .iter()
        .map(|i| quote! { blvm_node::module::traits::EventType::#i })
        .collect();

    let event_types_fn: ImplItem = syn::parse2(quote! {
        /// Event types to subscribe to (from #[on_event] handlers).
        pub fn event_types() -> Vec<blvm_node::module::traits::EventType> {
            vec![#(#event_type_exprs),*]
        }
    })
    .expect("event_types fn should parse");

    // Generate dispatch_event - match on event_type and call handler(s) with DI
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

    let dispatch_fn: ImplItem = syn::parse2(quote! {
        /// Dispatch event to #[on_event] handlers.
        pub async fn dispatch_event(
            &self,
            event: blvm_node::module::ipc::protocol::EventMessage,
        ) -> Result<(), blvm_node::module::traits::ModuleError> {
            use blvm_node::module::traits::EventType;
            match event.event_type {
                #(#match_arms),*
            }
            Ok(())
        }
    })
    .expect("dispatch_event fn should parse");

    impl_block.items.push(event_types_fn);
    impl_block.items.push(dispatch_fn);

    TokenStream::from(quote! { #impl_block })
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
            } else {
                payload_fields
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|(f, _)| f == name)
            }
        });

    if !use_di {
        return quote! { self.#method_ident(&event).await?; };
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

fn parse_on_event_args(attr: &syn::Attribute) -> Vec<syn::Ident> {
    let parser = Punctuated::<syn::Ident, Comma>::parse_terminated;
    attr.parse_args_with(parser)
        .map(|p| p.into_iter().collect())
        .unwrap_or_default()
}

/// Parameter attribute: `#[arg(long)]`, `#[arg(short = 'o')]`, `#[arg(default = "x")]`.
/// Use on CLI handler parameters for named-arg parsing and type coercion.
#[proc_macro_attribute]
pub fn arg(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Field attribute: `#[config_env]` or `#[config_env("ENV_NAME")]`.
/// Use with `#[module_config(name = "...")]` on the struct. Pass-through; the real logic
/// is in `module_config` which reads this attribute to generate `apply_env_overrides()`.
#[proc_macro_attribute]
pub fn config_env(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Placeholder derive macro for future use.
#[proc_macro_derive(ModuleCliSpec)]
pub fn derive_module_cli_spec(input: TokenStream) -> TokenStream {
    let _input = parse_macro_input!(input as DeriveInput);
    quote! {}.into()
}
