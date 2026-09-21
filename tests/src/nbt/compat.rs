use std::io::Cursor;

use temper_codec::decode::{NetDecode, NetDecodeOpts};
use temper_codec::encode::{NetEncode, NetEncodeOpts};
use temper_macros::{NBTDeserialize, NBTSerialize};
use temper_nbt::blob::NbtBlob;
use temper_nbt::NBT;

const TAG_END: u8 = 0;
const TAG_BYTE: u8 = 1;
const TAG_INT: u8 = 3;
const TAG_LONG: u8 = 4;
const TAG_BYTE_ARRAY: u8 = 7;
const TAG_STRING: u8 = 8;
const TAG_LIST: u8 = 9;
const TAG_COMPOUND: u8 = 10;
const TAG_INT_ARRAY: u8 = 11;
const TAG_LONG_ARRAY: u8 = 12;

#[derive(Debug, PartialEq, NBTSerialize, NBTDeserialize)]
struct ExistingFixture {
    title: String,
    count: i32,
    names: Vec<String>,
    nested: NestedFixture,
    bytes: Vec<i8>,
    ints: Vec<i32>,
    longs: Vec<i64>,
}

#[derive(Debug, PartialEq, NBTSerialize, NBTDeserialize)]
struct NestedFixture {
    enabled: bool,
    ticks: i64,
}

#[derive(Clone, Debug, PartialEq, NBTSerialize, NBTDeserialize)]
struct NetworkFixture {
    name: String,
    health: i32,
    flags: Vec<i8>,
    heights: Vec<i64>,
}

#[derive(Debug, PartialEq, NBTSerialize, NBTDeserialize)]
struct OptionalFixture {
    required: i32,
    missing: Option<String>,
    #[nbt(skip)]
    transient: String,
}

#[test]
fn derived_deserialize_reads_existing_named_root_payload() {
    let fixture = ExistingFixture::from_bytes(&existing_fixture_bytes()).unwrap();

    assert_eq!(fixture, existing_fixture());
}

#[test]
fn derived_serialize_round_trips_the_same_semantic_tree() {
    let original = existing_fixture();
    let bytes = original.serialize_with_header();
    let decoded = ExistingFixture::from_bytes(&bytes).unwrap();

    assert_eq!(decoded, original);
}

#[test]
fn optional_fields_can_be_absent_and_skipped_fields_use_default() {
    let mut bytes = Vec::new();
    compound_root(&mut bytes, "OptionalFixture", |bytes| {
        named_int(bytes, "required", 42);
    });

    let decoded = OptionalFixture::from_bytes(&bytes).unwrap();

    assert_eq!(
        decoded,
        OptionalFixture {
            required: 42,
            missing: None,
            transient: String::new(),
        }
    );
}

#[test]
fn nbt_generic_round_trips_network_payload_and_leaves_following_bytes() {
    let mut bytes = Vec::new();
    NBT::new(network_fixture())
        .encode(&mut bytes, &NetEncodeOpts::None)
        .unwrap();
    bytes.push(0x7f);

    let mut reader = Cursor::new(bytes);
    let decoded = NBT::<NetworkFixture>::decode(&mut reader, &NetDecodeOpts::None).unwrap();
    let marker = u8::decode(&mut reader, &NetDecodeOpts::None).unwrap();

    assert_eq!(*decoded, network_fixture());
    assert_eq!(marker, 0x7f);
}

#[test]
fn nbt_blob_preserves_one_network_payload_as_bytes() {
    let mut payload = Vec::new();
    NBT::new(network_fixture())
        .encode(&mut payload, &NetEncodeOpts::None)
        .unwrap();

    let mut stream = payload.clone();
    stream.push(0x2a);

    let mut reader = Cursor::new(stream);
    let blob = NbtBlob::decode(&mut reader, &NetDecodeOpts::None).unwrap();
    let marker = u8::decode(&mut reader, &NetDecodeOpts::None).unwrap();

    assert_eq!(blob.0, payload);
    assert_eq!(marker, 0x2a);
}

fn existing_fixture() -> ExistingFixture {
    ExistingFixture {
        title: "hello".to_string(),
        count: 7,
        names: vec!["alpha".to_string(), "beta".to_string()],
        nested: NestedFixture {
            enabled: true,
            ticks: 123_456_789,
        },
        bytes: vec![1, -2, 3],
        ints: vec![1, -2, 3],
        longs: vec![4, -5],
    }
}

fn network_fixture() -> NetworkFixture {
    NetworkFixture {
        name: "temper".to_string(),
        health: 20,
        flags: vec![1, 0, 1],
        heights: vec![64, 72, 80],
    }
}

fn existing_fixture_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    compound_root(&mut bytes, "ExistingFixture", |bytes| {
        named_string(bytes, "title", "hello");
        named_int(bytes, "count", 7);
        named_string_list(bytes, "names", &["alpha", "beta"]);
        named_compound(bytes, "nested", |bytes| {
            named_byte(bytes, "enabled", 1);
            named_long(bytes, "ticks", 123_456_789);
        });
        named_byte_array(bytes, "bytes", &[1, -2, 3]);
        named_int_array(bytes, "ints", &[1, -2, 3]);
        named_long_array(bytes, "longs", &[4, -5]);
    });
    bytes
}

fn compound_root(bytes: &mut Vec<u8>, name: &str, fields: impl FnOnce(&mut Vec<u8>)) {
    bytes.push(TAG_COMPOUND);
    string_payload(bytes, name);
    fields(bytes);
    bytes.push(TAG_END);
}

fn named_compound(bytes: &mut Vec<u8>, name: &str, fields: impl FnOnce(&mut Vec<u8>)) {
    named_header(bytes, TAG_COMPOUND, name);
    fields(bytes);
    bytes.push(TAG_END);
}

fn named_byte(bytes: &mut Vec<u8>, name: &str, value: i8) {
    named_header(bytes, TAG_BYTE, name);
    bytes.push(value as u8);
}

fn named_int(bytes: &mut Vec<u8>, name: &str, value: i32) {
    named_header(bytes, TAG_INT, name);
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn named_long(bytes: &mut Vec<u8>, name: &str, value: i64) {
    named_header(bytes, TAG_LONG, name);
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn named_string(bytes: &mut Vec<u8>, name: &str, value: &str) {
    named_header(bytes, TAG_STRING, name);
    string_payload(bytes, value);
}

fn named_string_list(bytes: &mut Vec<u8>, name: &str, values: &[&str]) {
    named_header(bytes, TAG_LIST, name);
    bytes.push(TAG_STRING);
    bytes.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        string_payload(bytes, value);
    }
}

fn named_byte_array(bytes: &mut Vec<u8>, name: &str, values: &[i8]) {
    named_header(bytes, TAG_BYTE_ARRAY, name);
    bytes.extend_from_slice(&(values.len() as i32).to_be_bytes());
    bytes.extend(values.iter().map(|value| *value as u8));
}

fn named_int_array(bytes: &mut Vec<u8>, name: &str, values: &[i32]) {
    named_header(bytes, TAG_INT_ARRAY, name);
    bytes.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
}

fn named_long_array(bytes: &mut Vec<u8>, name: &str, values: &[i64]) {
    named_header(bytes, TAG_LONG_ARRAY, name);
    bytes.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
}

fn named_header(bytes: &mut Vec<u8>, tag: u8, name: &str) {
    bytes.push(tag);
    string_payload(bytes, name);
}

fn string_payload(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u16).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
