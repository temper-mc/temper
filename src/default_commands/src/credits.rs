use bevy_ecs::prelude::Query;
use temper_codec::net_types::adhoc_id::AdHocID;
use temper_commands::Sender;
use temper_macros::command;
use temper_nbt::NBT;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::show_dialog::{DialogBody, DialogContent, ShowDialog};
use temper_text::TextComponent;

static CREDITS_TEXT: &str = include_str!("../../../assets/data/credits.txt");

#[command("credits")]
fn credits(#[sender] sender: Sender, query: Query<&StreamWriter>) {
    let conn = match sender {
        Sender::Server => {
            // Server cannot have credits
            return;
        }
        Sender::Player(entity) => query.get(entity).expect("sender does not exist"),
    };
    let lines = CREDITS_TEXT
        .lines()
        .map(|t| DialogBody {
            dialog_body_type: "minecraft:plain_message".to_string(),
            contents: TextComponent::from(t),
            width: Some(1024),
        })
        .collect::<Vec<_>>();
    let packet = ShowDialog {
        content: AdHocID::from(NBT::from(DialogContent {
            dialog_content_type: "minecraft:notice".to_string(),
            title: TextComponent::from("Credits"),
            body: lines,
        })),
    };
    conn.send_packet(packet).unwrap();
}
