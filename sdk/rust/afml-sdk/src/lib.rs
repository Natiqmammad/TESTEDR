use proc_macro::TokenStream;
use quote::quote;
use serde::Serialize;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[proc_macro_attribute]
pub fn afml_export(args: TokenStream, item: TokenStream) -> TokenStream {
    let metadata = syn::parse_macro_input!(args as syn::AttributeArgs);
    let sig = metadata
        .into_iter()
        .filter_map(|nested| {
            if let syn::NestedMeta::Meta(syn::Meta::NameValue(pair)) = nested {
                if pair.path.is_ident("signature") {
                    if let syn::Lit::Str(lit) = pair.lit {
                        return Some(lit.value());
                    }
                }
            }
            None
        })
        .next()
        .unwrap_or_else(|| "fn unknown()".into());

    if let Some(func_name) = func_ident(&item) {
        write_export(func_name, sig);
    }

    item
}

fn write_export(name: String, signature: String) {
    if let Ok(path) = std::env::var("AFML_EXPORTS_FILE") {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            let _ = create_dir_all(parent);
        }
        let entry = ExportEntry {
            name,
            signature,
            type_name: None,
            fields: vec![],
        };
        if let Ok(json) = serde_json::to_string(&entry) {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
                let _ = writeln!(file, "{}", json);
            }
        }
    }
}

fn func_ident(stream: &TokenStream) -> Option<String> {
    let item = syn::parse_macro_input::parse::<syn::ItemFn>(stream.clone().into()).ok()?;
    Some(item.sig.ident.to_string())
}

#[derive(Serialize)]
struct ExportEntry {
    name: String,
    signature: String,
    #[serde(rename = "type")]
    type_name: Option<String>,
    fields: Vec<Field>,
}

#[derive(Serialize)]
struct Field {
    name: String,
    ty: String,
}
