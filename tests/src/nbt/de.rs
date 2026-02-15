#![allow(dead_code)]
// since some structs are there just for testing purposes (fields aren't used and it gives stupid warnings)
#![cfg(test)]

use temper_macros::{NBTDeserialize, NBTSerialize};

#[test]
#[ignore]
fn test_derive() {
    #[derive(NBTSerialize)]
    struct BasicStruct {
        hello: String,
        world: Two,
        list: Vec<Three>,
    }

    #[derive(NBTSerialize)]
    struct Two {
        a: i32,
        b: i32,
        list: Vec<i32>,
    }

    #[derive(NBTSerialize, Debug)]
    struct Three {
        l: i32,
    }

    let some_struct = BasicStruct {
        hello: "Hello".to_string(),
        world: Two {
            a: 1,
            b: 2,
            list: vec![1, 2, 3],
        },
        list: vec![Three { l: 1 }, Three { l: 2 }],
    };

    let buffer = some_struct.serialize_with_header();

    let base_path = r#""../../../.etc/tests""#;
    std::fs::write(format!("{}/test_derive.nbt", base_path), buffer).unwrap();
}

#[test]
fn test_basic_derive_macro() {
    #[derive(NBTSerialize, NBTDeserialize, Debug)]
    struct HelloWorld {
        hello: Option<i32>,
        #[nbt(rename = "world69420")]
        world: i32,
    }

    let hello_world = HelloWorld {
        hello: None,
        world: 2,
    };

    let buffer = hello_world.serialize_with_header();

    let hello_world = HelloWorld::from_bytes(buffer.as_slice()).unwrap();

    assert_eq!(hello_world.hello, None);
}

#[test]
#[ignore]
fn test_bigtest_with_derive() {
    #[derive(NBTDeserialize, Debug)]
    struct BigTest {
        byte_test: i8,
        short_test: i16,
        int_test: i32,
        float_test: f32,
        long_test: i64,
        double_test: f64,
        string_test: String,
        byte_array_test: Vec<i8>,
        list_test_long: Vec<i64>,
        nested_compound_test: NestedCompound,
        list_test_compound: Vec<DatedValue>,
    }

    #[derive(NBTDeserialize, Debug)]
    struct NestedCompound {
        egg: NameValue,
        ham: NameValue,
    }

    #[derive(NBTDeserialize, Debug)]
    struct NameValue {
        name: String,
        value: f32,
    }

    #[derive(NBTDeserialize, Debug)]
    struct DatedValue {
        name: String,
        created_on: i64,
    }

    let data = include_bytes!("../../../.etc/bigtest.nbt");

    let big_test = BigTest::from_bytes(data).unwrap();

    println!("{:#?}", big_test);
}

#[test]
fn test_skip_field() {
    #[derive(NBTSerialize, NBTDeserialize, Debug)]
    struct SkipField {
        #[nbt(rename = "something else")]
        hello: i32,
        #[nbt(skip)]
        world: i32,
        some_optional: Option<i32>,
    }

    let struct_data = SkipField {
        hello: 1,
        world: 9372423,
        some_optional: Some(2),
    };

    let buffer = struct_data.serialize_with_header();

    let skip_field = SkipField::from_bytes(buffer.as_slice()).unwrap();

    assert_eq!(skip_field.hello, 1);
    assert_eq!(skip_field.world, 0);
}

#[test]
fn test_list_compound_derive() {
    #[derive(NBTSerialize, NBTDeserialize, Debug)]
    struct ListCompound {
        list: Vec<Compound>,
    }

    #[derive(NBTSerialize, NBTDeserialize, Debug)]
    struct Compound {
        a: i32,
        b: i32,
    }

    let list_compound = ListCompound {
        list: vec![Compound { a: 1, b: 2 }, Compound { a: 3, b: 4 }],
    };

    let buffer = list_compound.serialize_with_header();

    let list_compound = ListCompound::from_bytes(buffer.as_slice()).unwrap();

    assert_eq!(list_compound.list.len(), 2);
}
