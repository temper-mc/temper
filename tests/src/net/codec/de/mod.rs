use temper_codec::decode::NetDecode;
use temper_macros::NetDecode;

#[derive(NetDecode, Debug)]
#[expect(dead_code)]
struct SomeExampleEncStruct {
    pub field1: u32,
    pub field2: u32,
}

#[test]
fn test_decode() {
    let file = include_bytes!("../../../../../.etc/tests/enc_test_encode");

    let mut reader = std::io::Cursor::new(file);
    let example =
        SomeExampleEncStruct::decode(&mut reader, &temper_codec::decode::NetDecodeOpts::None)
            .unwrap();

    println!("{:?}", example);
}
