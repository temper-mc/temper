use std::process::{Command, Stdio};

use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};
use syn::__private::ToTokens;
use walkdir::WalkDir;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PacketId {
    protocol_id: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PacketDirection {
    #[serde(default)]
    clientbound: HashMap<String, PacketId>,
    #[serde(default)]
    serverbound: HashMap<String, PacketId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packets {
    configuration: PacketDirection,
    handshake: PacketDirection,
    login: PacketDirection,
    play: PacketDirection,
    status: PacketDirection,
}

#[derive(Clone, Copy)]
enum Bound {
    #[expect(dead_code)]
    Clientbound,
    Serverbound,
}

fn lookup_packet_id(packets: &Packets, state: &str, bound: Bound, packet_name: &str) -> u8 {
    let packet_name = packet_name.trim_matches('"');
    let key = if packet_name.starts_with("minecraft:") {
        packet_name.to_string()
    } else {
        format!("minecraft:{packet_name}")
    };

    let dir = match state {
        "configuration" => &packets.configuration,
        "handshake" => &packets.handshake,
        "login" => &packets.login,
        "play" => &packets.play,
        "status" => &packets.status,
        other => {
            panic!("Invalid state: {other}. Must be: configuration/handshake/login/play/status")
        }
    };

    let map = match bound {
        Bound::Clientbound => &dir.clientbound,
        Bound::Serverbound => &dir.serverbound,
    };

    map.get(&key)
        .unwrap_or_else(|| {
            panic!(
                "Could not find key `{key}` in packets.json (state={state}, bound={})",
                match bound {
                    Bound::Clientbound => "clientbound",
                    Bound::Serverbound => "serverbound",
                }
            )
        })
        .protocol_id
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i != 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

fn parse_packet_attr(attr: &syn::Attribute) -> Option<(String, String)> {
    if !attr.path().is_ident("packet") {
        return None;
    }

    let mut packet_id: Option<String> = None;
    let mut state: Option<String> = None;

    let meta = attr
        .parse_args_with(
            syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated,
        )
        .ok()?;
    for nv in meta {
        let ident = nv.path.get_ident()?.to_string();
        match ident.as_str() {
            "packet_id" => {
                packet_id = Some(match nv.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => s.value(),
                    other => other.to_token_stream().to_string().replace(' ', ""),
                });
            }
            "state" => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = nv.value
                {
                    state = Some(s.value());
                } else {
                    state = Some(nv.value.to_token_stream().to_string().replace(' ', ""));
                }
            }
            _ => {}
        }
    }

    Some((state?, packet_id?))
}

fn parse_packet_id_resolve(packets: &Packets, state: &str, bound: Bound, raw: &str) -> u8 {
    let s = raw.trim().trim_matches('"');

    if let Some(hex) = s.strip_prefix("0x") {
        return u8::from_str_radix(hex, 16)
            .unwrap_or_else(|e| panic!("bad hex packet_id `{s}`: {e}"));
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        return s
            .parse::<u8>()
            .unwrap_or_else(|e| panic!("bad decimal packet_id `{s}`: {e}"));
    }

    // named id
    lookup_packet_id(packets, state, bound, s)
}

fn main() {
    let child = Command::new("cargo")
        .args(["pkgid", "--package", "temper"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();

    let version = String::from_utf8_lossy(&child.stdout);

    let version = version.split('@').collect::<Vec<&str>>()[1];

    // Set env vars used for the server brand string
    println!("cargo:rustc-env=TEMPER_VERSION={}", version);
    println!(
        "cargo:rustc-env=BUILD_TYPE={}",
        if std::env::var("PROFILE").unwrap() == "debug" {
            " DEBUG"
        } else {
            ""
        }
    );

    let module_dir = "src/incoming";

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let input_dir = manifest_dir.join(module_dir);

    if !input_dir.is_dir() {
        panic!(
            "build.rs: module_dir is not a directory: {}",
            input_dir.display()
        );
    }
    let packets_json_path = manifest_dir.join("../../../assets/data/packets.json");
    if !packets_json_path.is_file() {
        panic!(
            "build.rs: missing packets.json at {}",
            packets_json_path.display()
        );
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", input_dir.display());
    println!("cargo:rerun-if-changed={}", packets_json_path.display());

    let packets: Packets = {
        let json = fs::read_to_string(&packets_json_path).unwrap_or_else(|e| {
            panic!(
                "build.rs: failed reading {}: {e}",
                packets_json_path.display()
            )
        });
        serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("build.rs: failed parsing packets.json: {e}"))
    };

    let base_mod_prefix = {
        let rel = Path::new(module_dir);
        let rel = rel.strip_prefix("src").unwrap_or(rel);
        let s = rel.to_string_lossy().replace(['\\', '/'], "::");
        if s.is_empty() {
            "crate".to_string()
        } else {
            format!("crate::{s}")
        }
    };

    let mut match_arms = String::new();
    let mut receiver_structs = String::new();
    let mut sender_fields = String::new();
    let mut send_recv_pairs = String::new();
    let mut build_sender_init = String::new();
    let mut register_resources = String::new();

    let bound = Bound::Serverbound;

    for entry in WalkDir::new(&input_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.extension().and_then(|x| x.to_str()) != Some("rs") {
            continue;
        }

        println!("cargo:rerun-if-changed={}", path.display());

        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("build.rs: failed reading {}: {e}", path.display()));
        let syntax = syn::parse_file(&content)
            .unwrap_or_else(|e| panic!("build.rs: syn parse failed for {}: {e}", path.display()));

        let mod_name = path.file_stem().unwrap().to_string_lossy().to_string();
        let module_path = format!("{base_mod_prefix}::{mod_name}");

        for item in syntax.items {
            let syn::Item::Struct(st) = item else {
                continue;
            };

            let mut found: Option<(String, u8)> = None;
            for attr in &st.attrs {
                let Some((state, packet_id_raw)) = parse_packet_attr(attr) else {
                    continue;
                };
                let id = parse_packet_id_resolve(&packets, &state, bound, &packet_id_raw);
                found = Some((state, id));
            }
            let Some((state, packet_id)) = found else {
                continue;
            };

            if state != "play" {
                continue;
            }
            let formatted_packet_id = format!("0x{packet_id:02X}u8");
            let struct_name = st.ident.to_string();
            let snake = to_snake_case(&struct_name);
            let struct_path = format!("{module_path}::{struct_name}");
            let receiver_name = format!("{struct_name}Receiver");

            match_arms.push_str(&format!(
                r#"{formatted_packet_id} => {{
            let packet = <{struct_path} as temper_codec::decode::NetDecode>::decode(cursor,&temper_codec::decode::NetDecodeOpts::None)?;
            if packet_sender.{snake}.is_full() {{
            tracing::trace!("Packet sender channel for {struct_name} is full. Dropping packet from {{}}.", entity);
            Ok(())
            }} else {{
            packet_sender.{snake}.send((packet, entity)).expect("Failed to send packet");
            Ok(())
            }}}},
            "#
            ));

            receiver_structs.push_str(&format!(
                r#"
#[derive(bevy_ecs::resource::Resource)]
pub struct {receiver_name}(pub crossbeam_channel::Receiver<({struct_path}, bevy_ecs::entity::Entity)>);
"#
            ));

            sender_fields.push_str(&format!(
                r#"
    pub {snake}: crossbeam_channel::Sender<({struct_path}, bevy_ecs::entity::Entity)>,
"#
            ));

            send_recv_pairs.push_str(&format!(
                r#"
    let ({snake}_sender, {snake}_receiver) = crossbeam_channel::bounded(250);
"#
            ));

            build_sender_init.push_str(&format!(
                r#"
        {snake}: {snake}_sender,
"#
            ));

            register_resources.push_str(&format!(
                r#"
    world.insert_resource({receiver_name}({snake}_receiver));
"#
            ));
        }
    }

    let generated = format!(
        r#"
// Autogenerated by a build script. Do not edit manually.

use std::sync::Arc;
use bevy_ecs::world::World;

pub fn handle_packet<R: std::io::Read>(
    packet_id: u8,
    entity: bevy_ecs::entity::Entity,
    cursor: &mut R,
    packet_sender: Arc<PacketSender>,
) -> Result<(), crate::errors::NetError> {{
    match packet_id {{
{match_arms}
        _ => {{
            tracing::debug!("No packet found for ID: 0x{{:02X}} (from {{}})", packet_id, entity);
            Err(crate::errors::PacketError::InvalidPacket(packet_id).into())
        }}
    }}
}}

{receiver_structs}

pub struct PacketSender {{
{sender_fields}
}}

pub fn create_packet_senders(world: &mut World) -> PacketSender {{
{send_recv_pairs}

    let packet_senders = PacketSender {{
{build_sender_init}
    }};

{register_resources}

    packet_senders
}}
"#
    );

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing"));
    let out_file = out_dir.join("packet_handling.rs");
    fs::write(&out_file, generated)
        .unwrap_or_else(|e| panic!("build.rs: failed writing {}: {e}", out_file.display()));
}
