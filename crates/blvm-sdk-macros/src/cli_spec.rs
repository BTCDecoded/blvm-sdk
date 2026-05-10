//! Generate CliSpec and dispatch_cli from #[cli_subcommand] impl block.
//!
//! Supports named args (--key value, -k value), type coercion, and #[arg(long, short)].

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{FnArg, ImplItem, ItemImpl, Pat, Type};

/// Extract #[arg(long)], #[arg(long = "x")], #[arg(short = 'o')], #[arg(default = "v")] from a param.
fn extract_arg_attrs(pt: &syn::PatType) -> (Option<String>, Option<String>, Option<String>) {
    let mut long_name = None;
    let mut short_name = None;
    let mut default = None;
    for attr in &pt.attrs {
        if !attr.path().is_ident("arg") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("long") {
                if meta.input.peek(syn::token::Eq) {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    long_name = Some(value.value());
                } else {
                    long_name = Some(String::new()); // use param name
                }
            } else if meta.path.is_ident("short") {
                let value: syn::LitChar = meta.value()?.parse()?;
                short_name = Some(value.value().to_string());
            } else if meta.path.is_ident("default") {
                let value: syn::LitStr = meta.value()?.parse()?;
                default = Some(value.value());
            }
            Ok(())
        });
    }
    (long_name, short_name, default)
}

/// Build CliSpec code from impl block methods.
pub fn generate_spec_code(item: &ItemImpl, cli_name: &str) -> TokenStream {
    let subcommands: Vec<TokenStream> = item
        .items
        .iter()
        .filter_map(|impl_item| {
            if let ImplItem::Fn(method) = impl_item {
                let method_name = method.sig.ident.to_string();
                // Skip generated helpers (no self)
                if method.sig.inputs.is_empty()
                    || method_name == "cli_spec"
                    || method_name == "dispatch_cli"
                {
                    return None;
                }
                let sub_name = to_kebab_case(&method_name);
                let about =
                    extract_doc_comment(method).unwrap_or_else(|| to_title_case(&method_name));

                let args = method
                    .sig
                    .inputs
                    .iter()
                    .filter_map(|arg| {
                        if let FnArg::Typed(pt) = arg {
                            if let Pat::Ident(pi) = &*pt.pat {
                                let name = pi.ident.to_string();
                                if name == "self" || name == "ctx" || name == "context" {
                                    return None;
                                }
                                if is_invocation_context_type(&pt.ty) {
                                    return None;
                                }
                                let (long_attr, short_attr, default_attr) = extract_arg_attrs(pt);
                                let long_name = match &long_attr {
                                    Some(s) if s.is_empty() => quote! { Some(#name.to_string()) },
                                    Some(s) => {
                                        let lit = proc_macro2::Literal::string(s);
                                        quote! { Some(#lit.to_string()) }
                                    }
                                    None => quote! { Some(#name.to_string()) },
                                };
                                let short_name = short_attr
                                    .as_ref()
                                    .map(|s| {
                                        let lit = proc_macro2::Literal::string(s);
                                        quote! { Some(#lit.to_string()) }
                                    })
                                    .unwrap_or(quote! { None });
                                let default = default_attr
                                    .as_ref()
                                    .map(|s| {
                                        let lit = proc_macro2::Literal::string(s);
                                        quote! { Some(#lit.to_string()) }
                                    })
                                    .unwrap_or(quote! { None });
                                let required = !is_option_type(&pt.ty);
                                let takes_value = true;
                                Some(quote! {
                                    blvm_node::module::ipc::protocol::CliArgSpec {
                                        name: #name.to_string(),
                                        long_name: #long_name,
                                        short_name: #short_name,
                                        required: Some(#required),
                                        takes_value: Some(#takes_value),
                                        default: #default,
                                    }
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                let args_tokens = if args.is_empty() {
                    quote! { vec![] }
                } else {
                    quote! { vec![#(#args),*] }
                };

                Some(quote! {
                    blvm_node::module::ipc::protocol::CliSubcommandSpec {
                        name: #sub_name.to_string(),
                        about: Some(#about.to_string()),
                        args: #args_tokens,
                    }
                })
            } else {
                None
            }
        })
        .collect();

    let cli_name_lit = cli_name;
    quote! {
        blvm_node::module::ipc::protocol::CliSpec {
            version: 1,
            name: #cli_name_lit.to_string(),
            about: None,
            subcommands: vec![#(#subcommands),*],
        }
    }
}

/// Generate coercion expr for a param: map -> value of type T.
fn coercion_expr(name: &str, ty: &Type, is_opt: bool) -> TokenStream {
    let name_lit = proc_macro2::Literal::string(name);
    let parse_expr = type_parse_expr(ty);
    if is_opt {
        quote! {
            map.get(#name_lit)
                .map(|s| #parse_expr)
                .transpose()
                .map_err(|e| blvm_node::module::traits::ModuleError::Other(e.into()))?
        }
    } else {
        quote! {
            map.get(#name_lit)
                .ok_or_else(|| blvm_node::module::traits::ModuleError::Other(
                    format!("missing required argument: {}", #name_lit).into()
                ))
                .and_then(|s| #parse_expr.map_err(|e| blvm_node::module::traits::ModuleError::Other(e.into())))?
        }
    }
}

/// Generate (|s| -> Result<T, String>) for parsing a string into the target type.
fn type_parse_expr(ty: &Type) -> TokenStream {
    let ty_str = ty.to_token_stream().to_string();
    let ty_compact = ty_str.replace(' ', "");
    if ty_compact == "bool" {
        quote! { blvm_sdk::module::cli_args::coerce_bool(s).map_err(|e| e.to_string()) }
    } else if ty_compact == "String" {
        quote! { Ok::<_, String>(s.to_string()) }
    } else if ty_compact == "i32" {
        quote! { s.parse::<i32>().map_err(|e| e.to_string()) }
    } else if ty_compact == "i64" {
        quote! { s.parse::<i64>().map_err(|e| e.to_string()) }
    } else if ty_compact == "u32" {
        quote! { s.parse::<u32>().map_err(|e| e.to_string()) }
    } else if ty_compact == "u64" {
        quote! { s.parse::<u64>().map_err(|e| e.to_string()) }
    } else if ty_compact.starts_with("Option<") {
        let inner = inner_type_if_option(ty);
        if let Some(inner_ty) = inner {
            type_parse_expr(&inner_ty)
        } else {
            quote! { Ok::<_, String>(s.to_string()) }
        }
    } else {
        quote! { s.parse().map_err(|e| e.to_string()) }
    }
}

/// Generate dispatch_cli with named-arg parsing and type coercion.
pub fn generate_dispatch_cli(item: &ItemImpl) -> Option<TokenStream> {
    let mut arms = Vec::new();

    for impl_item in &item.items {
        if let ImplItem::Fn(method) = impl_item {
            // Skip associated functions (no self) and generated helpers
            let name = method.sig.ident.to_string();
            if method.sig.inputs.is_empty() || name == "cli_spec" || name == "dispatch_cli" {
                continue;
            }
            let method_ident = &method.sig.ident;
            let method_name = method_ident.to_string();
            let sub_name = to_kebab_case(&method_name);

            let mut has_ctx = false;
            let mut param_infos = Vec::new();
            for arg in method.sig.inputs.iter().skip(1) {
                if let FnArg::Typed(pt) = arg {
                    if let Pat::Ident(pi) = &*pt.pat {
                        let name = pi.ident.to_string();
                        if name == "ctx" || name == "context" || is_invocation_context_type(&pt.ty)
                        {
                            has_ctx = true;
                            continue;
                        }
                        let (long_attr, short_attr, default_attr) = extract_arg_attrs(pt);
                        let is_opt = is_option_type(&pt.ty);
                        let inner_ty = inner_type_if_option(&pt.ty);
                        param_infos.push((
                            name,
                            long_attr,
                            short_attr,
                            default_attr,
                            pt.ty.clone(),
                            inner_ty,
                            is_opt,
                        ));
                    }
                }
            }

            let (arg_specs, arg_exprs): (Vec<_>, Vec<_>) = param_infos
                .iter()
                .map(
                    |(name, long_attr, short_attr, default_attr, ty, _inner, is_opt)| {
                        let long_name = match long_attr {
                            Some(s) if s.is_empty() => quote! { Some(#name.to_string()) },
                            Some(s) => {
                                let lit = proc_macro2::Literal::string(s);
                                quote! { Some(#lit.to_string()) }
                            }
                            None => quote! { Some(#name.to_string()) },
                        };
                        let short_name = short_attr
                            .as_ref()
                            .map(|s| {
                                let lit = proc_macro2::Literal::string(s);
                                quote! { Some(#lit.to_string()) }
                            })
                            .unwrap_or(quote! { None });
                        let default = default_attr
                            .as_ref()
                            .map(|s| {
                                let lit = proc_macro2::Literal::string(s);
                                quote! { Some(#lit.to_string()) }
                            })
                            .unwrap_or(quote! { None });
                        let spec = quote! {
                            blvm_node::module::ipc::protocol::CliArgSpec {
                                name: #name.to_string(),
                                long_name: #long_name,
                                short_name: #short_name,
                                required: Some(!#is_opt),
                                takes_value: Some(true),
                                default: #default,
                            }
                        };
                        let expr = coercion_expr(name, ty, *is_opt);
                        (spec, expr)
                    },
                )
                .unzip();

            let args_specs = if arg_specs.is_empty() {
                quote! { vec![] }
            } else {
                quote! { vec![#(#arg_specs),*] }
            };

            let call = if has_ctx {
                quote! { self.#method_ident(ctx, #(#arg_exprs),*) }
            } else {
                quote! { self.#method_ident(#(#arg_exprs),*) }
            };

            arms.push(quote! {
                #sub_name => {
                    let arg_specs = #args_specs;
                    let map = blvm_sdk::module::cli_args::parse_args(args, &arg_specs)
                        .map_err(|e| blvm_node::module::traits::ModuleError::Other(e.to_string().into()))?;
                    match #call {
                        Ok(msg) => Ok(msg),
                        Err(e) => Err(e.into()),
                    }
                }
            });
        }
    }

    if arms.is_empty() {
        return None;
    }

    Some(quote! {
        /// Dispatch CLI invocation to the matching subcommand handler.
        /// Supports named args (--key value), positional args, and type coercion.
        pub fn dispatch_cli(
            &self,
            ctx: &blvm_sdk::module::runner::InvocationContext,
            subcommand: &str,
            args: &[String],
        ) -> Result<String, blvm_node::module::traits::ModuleError> {
            match subcommand {
                #(#arms),*
                _ => Err(blvm_node::module::traits::ModuleError::Other(
                    format!("Unknown subcommand: {}", subcommand).into()
                )),
            }
        }
    })
}

fn inner_type_if_option(ty: &Type) -> Option<Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(t)) = args.args.first() {
                        return Some(t.clone());
                    }
                }
            }
        }
    }
    None
}

fn to_kebab_case(s: &str) -> String {
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
        .collect()
}

fn to_title_case(s: &str) -> String {
    let kebab = to_kebab_case(s);
    kebab
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().chain(c).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        let segs = &tp.path.segments;
        if let Some(last) = segs.last() {
            return last.ident == "Option";
        }
    }
    false
}

fn is_invocation_context_type(ty: &Type) -> bool {
    if let Type::Reference(tr) = ty {
        return is_invocation_context_type(&tr.elem);
    }
    if let Type::Path(tp) = ty {
        let segs = &tp.path.segments;
        if let Some(last) = segs.last() {
            return last.ident == "InvocationContext";
        }
    }
    false
}

fn extract_doc_comment(method: &syn::ImplItemFn) -> Option<String> {
    let mut docs = Vec::new();
    for attr in &method.attrs {
        if attr.path().is_ident("doc") {
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(el) = &nv.value {
                    if let syn::Lit::Str(s) = &el.lit {
                        let s = s.value().trim().to_string();
                        if !s.is_empty() {
                            docs.push(s);
                        }
                    }
                }
            }
        }
    }
    if docs.is_empty() {
        None
    } else {
        Some(docs.join(" "))
    }
}
