use std::collections::HashMap;
use temper_macros::NBTDeserialize;
use temper_nbt::{FromNbt, NBTSerializable, NBTSerializeOptions};

#[test]
fn basic_compound_ser() {
    let mut map = HashMap::new();
    map.insert("hello".to_string(), 42);

    let mut buf = Vec::new();
    map.serialize(&mut buf, &NBTSerializeOptions::WithHeader("test"));
}

#[test]
fn derive_macro_nested() {
    use temper_macros::NBTSerialize;

    #[derive(NBTSerialize)]
    struct Test {
        hello: i32,
        world: i32,
    }

    #[derive(NBTSerialize)]
    struct Test2 {
        test: Test,
    }

    let test = Test { hello: 1, world: 2 };

    let test2 = Test2 { test };

    let buf = test2.serialize_with_header();

    let mut parser = temper_nbt::de::borrow::NbtTape::new(&buf);
    parser.parse();

    let test = parser.get("test").unwrap();
    let hello = test.get("hello").unwrap();
    let world = test.get("world").unwrap();

    let hello = <i32 as FromNbt>::from_nbt(&parser, hello).unwrap();
    let world = <i32 as FromNbt>::from_nbt(&parser, world).unwrap();

    assert_eq!(hello, 1);
    assert_eq!(world, 2);
}

#[test]
fn very_basic_derive() {
    use temper_macros::NBTSerialize;

    // Define the struct
    #[derive(NBTSerialize, NBTDeserialize, PartialEq, Debug)]
    struct Test {
        hello: i32,
        world: i32,
    }

    // Create the struct
    let test = Test { hello: 1, world: 2 };

    // Serialize the struct
    let buf = test.serialize_with_header();

    // Deserialize the struct
    let test = Test::from_bytes(&buf).unwrap();

    assert_eq!(test, Test { hello: 1, world: 2 });
}
