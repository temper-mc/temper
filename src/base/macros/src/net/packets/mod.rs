use crate::static_loading::packets::{get_packet_id, PacketBoundiness};
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use regex::Regex;
use syn::Attribute;

/// Returns: (state, packet_id)
fn parse_packet_attribute(attr: &Attribute) -> Option<(String, String)> {
    let attr_str = attr.to_token_stream().to_string();

    // This regex matches both formats:
    // #[packet(packet_id = "something", state = "play")]
    let re = Regex::new(r#"packet_id\s*=\s*"([^"]+)"(?:\s*,\s*)?state\s*=\s*"([^"]+)""#).unwrap();

    if let Some(caps) = re.captures(&attr_str) {
        let packet_id = caps.get(1).map(|m| m.as_str().to_string())?;
        let state = caps.get(2).map(|m| m.as_str().to_string())?;
        Some((state, packet_id))
    } else {
        None
    }
}

/// Returns: (state, packet_id)
pub(crate) fn get_packet_details_from_attributes(
    attrs: &[Attribute],
    bound_to: PacketBoundiness,
) -> Option<(String, u8)> {
    let mut val = Option::<(String, String)>::None;

    for attr in attrs {
        if !attr.path().is_ident("packet") {
            continue;
        }

        val = parse_packet_attribute(attr);
    }

    let (state, packet_id) = val?;

    let packet_id =
        parse_packet_id(state.as_str(), packet_id, bound_to).expect("parse_packet_id failed");

    Some((state, packet_id))
}

fn parse_packet_id(state: &str, value: String, bound_to: PacketBoundiness) -> syn::Result<u8> {
    //! Sorry to anyone reading this code. The get_packet_id method PANICS if there is any type of error.
    //! these macros are treated like trash gah damn. they need better care 😔

    // If the user provided a direct integer (like 0x01, or any number) value.
    if value.starts_with("0x") {
        let value = value.strip_prefix("0x").expect("strip_prefix failed");
        let n = u8::from_str_radix(value, 16).expect("from_str_radix failed");
        return Ok(n);
    }

    // If the user provided referencing packet id, then just get that.
    let n = get_packet_id(state, bound_to, value.as_str());

    Ok(n)
}

/// `#[packet]` attribute is used to declare an incoming/outgoing packet.
///
/// <b>packet_id</b> => The packet id of the packet. In hexadecimal.
/// <b>state</b> => The state of the packet. Can be: "handshake", "status", "login", "play".
///
/// e.g.
/// ```ignore
/// use temper_macros::NetDecode;
///
/// #[derive(NetDecode)]
/// #[packet(packet_id = 0x05, state = "play")]
/// pub struct PacketChatMessage {
///     pub message: String,
///     pub timestamp: i64,
/// }
/// ```
///
/// ```ignore
/// use temper_macros::{packet, NetEncode};
///
/// #[derive(NetEncode)]
/// #[packet(packet_id = 0x05)]
/// pub struct PacketChatMessage {
///    pub message: String,
///    pub timestamp: i64,
/// }
pub fn attribute(args: TokenStream, input: TokenStream) -> TokenStream {
    // These are just some checks to make sure the packet attribute is used correctly.
    // This is not actual functionality.
    // The actual functionality is in the `bake_registry` function.

    const E: &str = "packet attribute must have the packet_id and/or state fields. In case of incoming: both. In case of outgoing: only packet_id.";
    if args.is_empty() {
        return TokenStream::from(quote! {
            compile_error!(#E);
        });
    }

    if !&["packet_id", "state"]
        .iter()
        .all(|x| args.to_string().contains(x))
    {
        return TokenStream::from(quote! {
            compile_error!(#E);
        });
    }

    input
}
