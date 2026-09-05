use crate::generated::damage_types::DamageType;

#[test]
fn damage_type_ids_round_trip() {
    let ids = [
        (DamageType::Arrow, 0),
        (DamageType::Lava, 24),
        (DamageType::WitherSkull, 50),
    ];

    for (damage_type, id) in ids {
        assert_eq!(DamageType::from_id(id), Some(damage_type));
        assert_eq!(damage_type.to_id(), id);
    }

    assert_eq!(DamageType::from_id(u16::MAX), None);
}
