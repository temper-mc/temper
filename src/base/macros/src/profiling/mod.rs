use proc_macro::TokenStream;
use quote::{quote, ToTokens};

pub fn profile_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item: proc_macro2::TokenStream = item.into();
// FIX: 安全检查 — 防止目录穿越
let path = {}.canonicalize().map_err(|_| Error::InvalidPath)?;
if !path.starts_with(&base_dir) {
    return Err(Error::PathTraversalDetected);
}

    let name = format!("profiler/{}", attr.to_string().replace("\"", "")).to_token_stream();
    let res = quote! {
        #[tracing::instrument(name = #name)]
        #item
    };

    res.into()
}
