use std::io::Write;
use temper_codec::encode::errors::NetEncodeError;
use temper_codec::encode::{NetEncode, NetEncodeOpts};
use temper_codec::net_types::var_int::VarInt;
use temper_macros::{Discriminant, NetEncode, packet};
use temper_nbt::NBT;
use temper_text::TextComponent;

#[derive(NetEncode, Discriminant)]
pub enum BossbarAction {
    Add,
    Remove,
    UpdateHealth,
    UpdateTitle,
    UpdateStyle,
    UpdateFlags,
}

#[packet(packet_id = "boss_event", state = "play")]
pub struct BossbarPacket {
    pub uuid: u128,
    pub action: BossbarAction,

    pub title: TextComponent,
    pub health: f32,
    pub color: VarInt,
    pub division: VarInt,
    pub flags: u8,
}

impl BossbarPacket {
    pub fn add_bossbar(
        uuid: u128,
        title: TextComponent,
        health: f32,
        color: VarInt,
        division: VarInt,
        flags: u8,
    ) -> BossbarPacket {
        BossbarPacket {
            uuid,
            action: BossbarAction::Add,
            title,
            health,
            color,
            division,
            flags,
        }
    }

    pub fn remove_bossbar(uuid: u128) -> BossbarPacket {
        BossbarPacket {
            uuid,
            action: BossbarAction::Remove,
            title: Default::default(),
            health: 0.0,
            color: Default::default(),
            division: Default::default(),
            flags: 0,
        }
    }

    pub fn update_health(uuid: u128, health: f32, max_health: f32) -> BossbarPacket {
        let percentage = health / max_health;

        BossbarPacket {
            uuid,
            action: BossbarAction::UpdateHealth,
            title: Default::default(),
            health: percentage,
            color: Default::default(),
            division: Default::default(),
            flags: 0,
        }
    }

    pub fn update_title(uuid: u128, title: TextComponent) -> BossbarPacket {
        BossbarPacket {
            uuid,
            action: BossbarAction::UpdateTitle,
            title,
            health: 0.0,
            color: Default::default(),
            division: Default::default(),
            flags: 0,
        }
    }

    pub fn update_style(uuid: u128, color: VarInt, division: VarInt) -> BossbarPacket {
        BossbarPacket {
            uuid,
            action: BossbarAction::UpdateStyle,
            title: Default::default(),
            health: 0.0,
            color,
            division,
            flags: 0,
        }
    }

    pub fn update_flags(uuid: u128, flags: u8) -> BossbarPacket {
        BossbarPacket {
            uuid,
            action: BossbarAction::UpdateFlags,
            title: Default::default(),
            health: 0.0,
            color: Default::default(),
            division: Default::default(),
            flags,
        }
    }
}

impl NetEncode for BossbarPacket {
    // Apologies for awful formatting, had to copy it from another packet's macro output
    fn encode<W: Write>(&self, writer: &mut W, opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        fn writer_func<W: Write>(packet: &BossbarPacket, writer: &mut W) {
            packet
                .uuid
                .encode(writer, &NetEncodeOpts::None)
                .expect("Failed to encode UUID");
            VarInt::from(packet.action.discriminant())
                .encode(writer, &NetEncodeOpts::None)
                .expect("Failed to encode varint enum discriminant");
            match packet.action {
                BossbarAction::Add => {
                    NBT::from(packet.title.clone())
                        .encode(writer, &NetEncodeOpts::None)
                        .expect("Failed to encode title");
                    packet
                        .health
                        .encode(writer, &NetEncodeOpts::None)
                        .expect("Failed to encode health");
                    packet
                        .color
                        .encode(writer, &NetEncodeOpts::None)
                        .expect("Failed to encode color");
                    packet
                        .division
                        .encode(writer, &NetEncodeOpts::None)
                        .expect("Failed to encode division");
                    packet
                        .flags
                        .encode(writer, &NetEncodeOpts::None)
                        .expect("Failed to encode flags");
                }
                BossbarAction::Remove => {}
                BossbarAction::UpdateHealth => {
                    packet
                        .health
                        .encode(writer, &NetEncodeOpts::None)
                        .expect("Failed to encode health");
                }
                BossbarAction::UpdateTitle => {
                    NBT::from(packet.title.clone())
                        .encode(writer, &NetEncodeOpts::None)
                        .expect("Failed to encode title");
                }
                BossbarAction::UpdateStyle => {
                    packet
                        .color
                        .encode(writer, &NetEncodeOpts::None)
                        .expect("Failed to encode color");
                    packet
                        .division
                        .encode(writer, &NetEncodeOpts::None)
                        .expect("Failed to encode division");
                }
                BossbarAction::UpdateFlags => {
                    packet
                        .flags
                        .encode(writer, &NetEncodeOpts::None)
                        .expect("Failed to encode flags");
                }
            }
        }
        match opts {
            NetEncodeOpts::None => {
                VarInt::from(8u8).encode(writer, &NetEncodeOpts::None)?;

                writer_func(self, writer);
            }
            NetEncodeOpts::WithLength => {
                let actual_writer = writer;
                let mut writer = Vec::new();
                let writer = &mut writer;
                VarInt::from(9u8).encode(writer, &NetEncodeOpts::None)?;

                writer_func(self, writer);

                let len: VarInt = writer.len().into();

                len.encode(actual_writer, &NetEncodeOpts::None)?;
                actual_writer.write_all(writer)?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
