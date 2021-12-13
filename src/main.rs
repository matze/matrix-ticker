use anyhow::{Context, Result};
use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::Text;
use embedded_text::style::{HeightMode, TextBoxStyleBuilder};
use embedded_text::TextBox;
use matrix_sdk::room::Room;
use matrix_sdk::ruma::UserId;
use matrix_sdk::ruma::events::room::message::{MessageEventContent, MessageType};
use matrix_sdk::ruma::events::SyncMessageEvent;
use matrix_sdk::{Client, SyncSettings};
use serde_derive::Deserialize;
use sh1106::displaysize::DisplaySize;
use sh1106::mode::GraphicsMode;
use tokio::sync::mpsc;

#[derive(Deserialize)]
struct Config {
    user_id: String,
    password: String,
}

struct Message {
    sender: String,
    content: String,
}

async fn on_room_message(
    event: SyncMessageEvent<MessageEventContent>,
    room: Room,
    tx: mpsc::Sender<Message>,
) -> matrix_sdk::Result<()> {
    match event.content.msgtype {
        MessageType::Text(content) => {
            let member = room.get_member(&event.sender).await?.unwrap();
            let _ = tx
                .send(Message {
                    content: content.body,
                    sender: member
                        .display_name()
                        .unwrap_or_else(|| member.user_id().localpart())
                        .to_string(),
                })
                .await;
        }
        _ => {}
    }

    Ok(())
}

async fn run_display_loop(mut rx: mpsc::Receiver<Message>) -> Result<()> {
    let i2c = rppal::i2c::I2c::new().context("Unable to create I2c object")?;

    let mut display: GraphicsMode<_> = sh1106::builder::Builder::new()
        .with_size(DisplaySize::Display128x64)
        .connect_i2c(i2c)
        .into();

    display.init().unwrap();
    display.flush().unwrap();

    let char_style = MonoTextStyle::new(&ascii::FONT_5X7, BinaryColor::On);
    let textbox_style = TextBoxStyleBuilder::new()
        .height_mode(HeightMode::FitToText)
        .build();

    let bounds = Rectangle::new(Point::new(0, 9), Size::new(128, 0));

    while let Some(message) = rx.recv().await {
        display.clear();

        let sender = message.sender.to_ascii_uppercase();

        Text::new(&sender, Point::new(0, 7), char_style).draw(&mut display)?;

        TextBox::with_textbox_style(&message.content, bounds, char_style, textbox_style)
            .draw(&mut display)?;

        display.flush().unwrap();
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config: Config = toml::from_str(&std::fs::read_to_string("config.toml")?)?;

    let user_id: UserId = config.user_id.clone().try_into()?;
    let client = Client::new_from_user_id(user_id).await?;

    let (tx, rx) = mpsc::channel(32);

    client
        .login(&config.user_id, &config.password, Some("pi-zero"), None)
        .await?;

    client
        .register_event_handler(move |ev, room| on_room_message(ev, room, tx.clone()))
        .await;

    let sync_loop = client.sync(SyncSettings::new());
    let display_loop = run_display_loop(rx);

    tokio::join!(sync_loop, display_loop);

    Ok(())
}
