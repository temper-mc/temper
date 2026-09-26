use crate::de::borrow::{NbtTape, NbtTapeElement};
use crate::{NBTError, Result};

pub trait FromNbt<'a>: Sized {
    fn from_nbt<'tape>(
        tapes: &'tape NbtTape<'a>,
        element: NbtTapeElement<'a, 'tape>,
    ) -> Result<Self>
    where
        'a: 'tape;
}

mod primitives {
    use super::*;
    use crate::de::borrow::NbtDeserializable;

    macro_rules! impl_for_primitives {
        ($($ty:ty) | *, $variant:ident) => {
            $(
            impl<'a> FromNbt<'a> for $ty {
                fn from_nbt<'tape>(
                    _tapes: &'tape NbtTape<'a>,
                    element: NbtTapeElement<'a, 'tape>,
                ) -> Result<Self>
                where
                    'a: 'tape,
                {
                    match element {
                        NbtTapeElement::$variant(val) => Ok(val as $ty),
                        _ => Err(NBTError::TypeMismatch { expected: stringify!($variant), found: element.nbt_type() }),
                    }
                }
            })*
        };
    }

    impl_for_primitives!(i8 | u8, Byte);
    impl_for_primitives!(i16 | u16, Short);
    impl_for_primitives!(i32 | u32, Int);
    impl_for_primitives!(i64 | u64, Long);
    impl_for_primitives!(f32, Float);
    impl_for_primitives!(f64, Double);

    impl<'a> FromNbt<'a> for bool {
        fn from_nbt<'tape>(
            _tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            match element {
                NbtTapeElement::Byte(val) => Ok(val != 0),
                _ => Err(NBTError::TypeMismatch {
                    expected: "Byte",
                    found: element.nbt_type(),
                }),
            }
        }
    }

    impl<'a> FromNbt<'a> for String {
        fn from_nbt<'tape>(
            _tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            match element {
                NbtTapeElement::String(val) => Ok(val.to_str().into_owned()),
                _ => Err(NBTError::TypeMismatch {
                    expected: "String",
                    found: element.nbt_type(),
                }),
            }
        }
    }

    impl<'a> FromNbt<'a> for &'a str {
        fn from_nbt<'tape>(
            _tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            Err(NBTError::TypeMismatch {
                expected: "String (owned String is required for MUTF-8 NBT)",
                found: element.nbt_type(),
            })
        }
    }

    impl<'a, T: FromNbt<'a>> FromNbt<'a> for Vec<T> {
        fn from_nbt<'tape>(
            tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            let found = element.nbt_type();
            match tapes.unpack_list::<T>(element) {
                Some(vec) => Ok(vec),
                None => Err(NBTError::TypeMismatch {
                    expected: "List",
                    found,
                }),
            }
        }
    }

    impl<'a, T: NbtDeserializable<'a>> FromNbt<'a> for &'a [T] {
        fn from_nbt<'tape>(
            tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            let found = element.nbt_type();
            match tapes.unpack_list_sliced::<T>(&element) {
                Some(slice) => Ok(slice),
                None => Err(NBTError::TypeMismatch {
                    expected: "List Slice (T != array type)",
                    found,
                }),
            }
        }
    }

    // optional
    impl<'a, T: FromNbt<'a>> FromNbt<'a> for Option<T> {
        fn from_nbt<'tape>(
            tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            // handle optionals yourself lol (jk they're handled by the derive macro :p)
            Ok(Some(T::from_nbt(tapes, element)?))
        }
    }

    impl<'a, T: FromNbt<'a>> FromNbt<'a> for Box<T> {
        fn from_nbt<'tape>(
            tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            Ok(Box::new(T::from_nbt(tapes, element)?))
        }
    }
}

mod maps {
    use crate::{FromNbt, NBTError, NbtTape, NbtTapeElement, Result};
    use std::collections::{BTreeMap, HashMap};

    impl<'a, V: FromNbt<'a>> FromNbt<'a> for HashMap<String, V> {
        fn from_nbt<'tape>(
            tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            let NbtTapeElement::Compound(compound) = element else {
                return Err(NBTError::TypeMismatch {
                    expected: "Compound (from HashMap<String, V>)",
                    found: element.nbt_type(),
                });
            };
            compound
                .iter()
                .map(|(key, val)| {
                    Ok((
                        key.to_str().into_owned(),
                        V::from_nbt(
                            tapes,
                            crate::de::borrow::convert_tag(val).ok_or(NBTError::InvalidNBTData)?,
                        )?,
                    ))
                })
                .collect()
        }
    }

    impl<'a, V: FromNbt<'a>> FromNbt<'a> for HashMap<&'a str, V> {
        fn from_nbt<'tape>(
            _tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            let _compound = element.as_compound().ok_or(NBTError::TypeMismatch {
                expected: "Compound (from HashMap<&str, V>, use HashMap<String, V>)",
                found: element.nbt_type(),
            })?;
            Err(NBTError::InvalidNBTData)
        }
    }

    impl<'a, V: FromNbt<'a>> FromNbt<'a> for BTreeMap<&'a str, V> {
        fn from_nbt<'tape>(
            _tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            let _compound = element.as_compound().ok_or(NBTError::TypeMismatch {
                expected: "Compound (from BTreeMap<&str, V>, use BTreeMap<String, V>)",
                found: element.nbt_type(),
            })?;
            Err(NBTError::InvalidNBTData)
        }
    }

    impl<'a, V: FromNbt<'a>> FromNbt<'a> for BTreeMap<String, V> {
        fn from_nbt<'tape>(
            tapes: &'tape NbtTape<'a>,
            element: NbtTapeElement<'a, 'tape>,
        ) -> Result<Self>
        where
            'a: 'tape,
        {
            let NbtTapeElement::Compound(compound) = element else {
                return Err(NBTError::TypeMismatch {
                    expected: "Compound (from BTreeMap<String, V>)",
                    found: element.nbt_type(),
                });
            };
            compound
                .iter()
                .map(|(key, val)| {
                    Ok((
                        key.to_str().into_owned(),
                        V::from_nbt(
                            tapes,
                            crate::de::borrow::convert_tag(val).ok_or(NBTError::InvalidNBTData)?,
                        )?,
                    ))
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod test_map {
    use crate::{FromNbt, NBTSerializable, NBTSerializeOptions};
    use std::collections::{BTreeMap, HashMap};

    #[test]
    fn test_hashmap_both_ways() {
        let some_hashmap = HashMap::from([
            ("key1".to_string(), 1),
            ("key2".to_string(), 2),
            ("key3".to_string(), 3),
        ]);

        let data = {
            let mut buf = Vec::new();
            some_hashmap.serialize(&mut buf, &NBTSerializeOptions::WithHeader("root"));
            buf
        };

        let mut tapes = crate::de::borrow::NbtTape::new(&data);
        tapes.parse();
        let root = tapes.root_element().expect("failed to get root");
        let hashmap =
            HashMap::<String, i32>::from_nbt(&tapes, root).expect("failed to deserialize root");

        assert_eq!(some_hashmap, hashmap);
    }

    #[test]
    fn test_btreemap_both_ways() {
        let some_btreemap = BTreeMap::from([
            ("key1".to_string(), 1),
            ("key2".to_string(), 2),
            ("key3".to_string(), 3),
        ]);

        let data = {
            let mut buf = Vec::new();
            some_btreemap.serialize(&mut buf, &NBTSerializeOptions::WithHeader("root"));
            buf
        };

        let mut tapes = crate::de::borrow::NbtTape::new(&data);
        tapes.parse();
        let root = tapes.root_element().expect("failed to get root");
        let btreemap = BTreeMap::<String, i32>::from_nbt(&tapes, root.clone())
            .expect("failed to deserialize root");

        assert_eq!(some_btreemap, btreemap);
    }

    #[test]
    fn borrowed_map_keys_are_rejected() {
        let some_hashmap = HashMap::from([("key".to_string(), 1)]);

        let data = {
            let mut buf = Vec::new();
            some_hashmap.serialize(&mut buf, &NBTSerializeOptions::WithHeader("root"));
            buf
        };

        let mut tapes = crate::de::borrow::NbtTape::new(&data);
        tapes.parse();
        let root = tapes.root_element().expect("failed to get root");

        assert!(HashMap::<&str, i32>::from_nbt(&tapes, root.clone()).is_err());
        assert!(BTreeMap::<&str, i32>::from_nbt(&tapes, root.clone()).is_err());
    }
}
