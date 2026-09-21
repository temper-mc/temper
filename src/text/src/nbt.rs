use crate::{ClickEvent, Color, Font, HoverEvent, NamedColor, TextComponent, TextContent};
use std::io::Read;
use temper_codec::decode::errors::NetDecodeError;
use temper_codec::decode::{NetDecode, NetDecodeOpts};
use temper_nbt::{FromNbt, NBTError, NbtTape, NbtTapeElement, NBT};

impl NetDecode for TextComponent {
    fn decode<R: Read>(reader: &mut R, opts: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
        let component = NBT::<TextComponent>::decode(reader, opts)?;
        Ok((*component).clone())
    }
}

impl<'a> FromNbt<'a> for TextComponent {
    fn from_nbt(tapes: &NbtTape<'a>, element: NbtTapeElement<'a>) -> temper_nbt::Result<Self> {
        Ok(Self {
            content: TextContent::from_nbt(tapes, element.clone())?,
            color: optional(tapes, &element, "color")?,
            font: optional(tapes, &element, "font")?,
            bold: optional(tapes, &element, "bold")?,
            italic: optional(tapes, &element, "italic")?,
            underlined: optional(tapes, &element, "underlined")?,
            strikethrough: optional(tapes, &element, "strikethrough")?,
            obfuscated: optional(tapes, &element, "obfuscated")?,
            insertion: optional(tapes, &element, "insertion")?,
            click_event: optional(tapes, &element, "click_event")?,
            hover_event: optional(tapes, &element, "hover_event")?,
            extra: optional(tapes, &element, "extra")?.unwrap_or_default(),
        })
    }
}

impl<'a> FromNbt<'a> for TextContent {
    fn from_nbt(tapes: &NbtTape<'a>, element: NbtTapeElement<'a>) -> temper_nbt::Result<Self> {
        if let Some(text) = element.get("text") {
            return Ok(Self::Text {
                text: String::from_nbt(tapes, text)?,
            });
        }

        if let Some(translate) = element.get("translate") {
            return Ok(Self::Translate {
                translate: String::from_nbt(tapes, translate)?,
                with: optional(tapes, &element, "with")?.unwrap_or_default(),
            });
        }

        if let Some(keybind) = element.get("keybind") {
            return Ok(Self::Keybind {
                keybind: String::from_nbt(tapes, keybind)?,
            });
        }

        Err(NBTError::ElementNotFound("text content"))
    }
}

impl FromNbt<'_> for Color {
    fn from_nbt(_tapes: &NbtTape, element: NbtTapeElement<'_>) -> temper_nbt::Result<Self> {
        let value = string_value(element)?;

        Ok(match named_color(&value) {
            Some(color) => Self::Named(color),
            None => Self::Hex(value),
        })
    }
}

impl FromNbt<'_> for NamedColor {
    fn from_nbt(_tapes: &NbtTape, element: NbtTapeElement<'_>) -> temper_nbt::Result<Self> {
        named_color(&string_value(element)?).ok_or(NBTError::InvalidNBTData)
    }
}

impl FromNbt<'_> for Font {
    fn from_nbt(_tapes: &NbtTape, element: NbtTapeElement<'_>) -> temper_nbt::Result<Self> {
        Ok(match string_value(element)?.as_str() {
            "minecraft:default" => Self::Default,
            "minecraft:uniform" => Self::Uniform,
            "minecraft:alt" => Self::Alt,
            custom => Self::Custom(custom.to_string()),
        })
    }
}

impl<'a> FromNbt<'a> for ClickEvent {
    fn from_nbt(tapes: &NbtTape<'a>, element: NbtTapeElement<'a>) -> temper_nbt::Result<Self> {
        let action = required::<String>(tapes, &element, "action")?;
        let value = required_element(&element, "value")?;

        match action.as_str() {
            "open_url" => Ok(Self::OpenUrl(String::from_nbt(tapes, value)?)),
            "run_command" => Ok(Self::RunCommand(String::from_nbt(tapes, value)?)),
            "suggest_command" => Ok(Self::SuggestCommand(String::from_nbt(tapes, value)?)),
            "change_page" => Ok(Self::ChangePage(i32::from_nbt(tapes, value)?)),
            "copy_to_clipboard" => Ok(Self::CopyToClipboard(String::from_nbt(tapes, value)?)),
            _ => Err(NBTError::InvalidNBTData),
        }
    }
}

impl<'a> FromNbt<'a> for HoverEvent {
    fn from_nbt(tapes: &NbtTape<'a>, element: NbtTapeElement<'a>) -> temper_nbt::Result<Self> {
        let action = required::<String>(tapes, &element, "action")?;
        let value = required_element(&element, "value")?;

        match action.as_str() {
            "show_text" => Ok(Self::ShowText(Box::<TextComponent>::from_nbt(
                tapes,
                value,
            )?)),
            "show_item" => Ok(Self::ShowItem {
                id: required(tapes, &value, "id")?,
                count: required(tapes, &value, "count")?,
                tag: required(tapes, &value, "tag")?,
            }),
            "show_entity" => Ok(Self::ShowEntity {
                entity_type: required(tapes, &value, "type")?,
                id: parse_uuid(required::<String>(tapes, &value, "id")?)?,
                name: optional(tapes, &value, "name")?,
            }),
            _ => Err(NBTError::InvalidNBTData),
        }
    }
}

fn optional<'a, T: FromNbt<'a>>(
    tapes: &NbtTape<'a>,
    element: &NbtTapeElement<'a>,
    key: &'static str,
) -> temper_nbt::Result<Option<T>> {
    element
        .get(key)
        .map(|value| T::from_nbt(tapes, value))
        .transpose()
}

fn required<'a, T: FromNbt<'a>>(
    tapes: &NbtTape<'a>,
    element: &NbtTapeElement<'a>,
    key: &'static str,
) -> temper_nbt::Result<T> {
    T::from_nbt(tapes, required_element(element, key)?)
}

fn required_element<'a, 'b>(
    element: &'b NbtTapeElement<'a>,
    key: &'static str,
) -> temper_nbt::Result<NbtTapeElement<'a>> {
    element.get(key).ok_or(NBTError::ElementNotFound(key))
}

fn string_value(element: NbtTapeElement<'_>) -> temper_nbt::Result<String> {
    match element {
        NbtTapeElement::String(value) => Ok(value),
        _ => Err(NBTError::TypeMismatch {
            expected: "String",
            found: element.nbt_type(),
        }),
    }
}

fn named_color(value: &str) -> Option<NamedColor> {
    match value {
        "black" => Some(NamedColor::Black),
        "dark_blue" => Some(NamedColor::DarkBlue),
        "dark_green" => Some(NamedColor::DarkGreen),
        "dark_aqua" => Some(NamedColor::DarkAqua),
        "dark_red" => Some(NamedColor::DarkRed),
        "dark_purple" => Some(NamedColor::DarkPurple),
        "gold" => Some(NamedColor::Gold),
        "gray" => Some(NamedColor::Gray),
        "dark_gray" => Some(NamedColor::DarkGray),
        "blue" => Some(NamedColor::Blue),
        "green" => Some(NamedColor::Green),
        "aqua" => Some(NamedColor::Aqua),
        "red" => Some(NamedColor::Red),
        "light_purple" => Some(NamedColor::LightPurple),
        "yellow" => Some(NamedColor::Yellow),
        "white" => Some(NamedColor::White),
        _ => None,
    }
}

fn parse_uuid(value: String) -> temper_nbt::Result<uuid::Uuid> {
    uuid::Uuid::parse_str(&value).map_err(|_| NBTError::InvalidNBTData)
}
