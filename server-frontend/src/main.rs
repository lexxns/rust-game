use bevy::{
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
};
use std::mem;

#[derive(Component)]
struct ChatHistory {
    messages: Vec<String>,
    max_messages: usize,
}

#[derive(Component)]
struct ChatInput;

#[derive(Component)]
struct ChatHistoryText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (listen_keyboard_input_events, update_chat_display))
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    // Chat history text display
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: font.clone(),
            font_size: 24.0,
            ..default()
        },
        Transform::from_xyz(10.0, 120.0, 0.0),
        ChatHistoryText,
    ));

    // Chat input text
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font,
            font_size: 24.0,
            ..default()
        },
        Transform::from_xyz(10.0, 10.0, 0.0),
        ChatInput,
    ));

    // Initialize chat history
    commands.spawn(ChatHistory {
        messages: Vec::new(),
        max_messages: 10, // Keep last 10 messages
    });
}

fn listen_keyboard_input_events(
    mut events: EventReader<KeyboardInput>,
    mut chat_input: Query<&mut Text2d, With<ChatInput>>,
    mut chat_history: Query<&mut ChatHistory>,
) {
    let mut input_text = chat_input.single_mut();
    let mut history = chat_history.single_mut();

    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match &event.logical_key {
            Key::Enter => {
                if input_text.is_empty() {
                    continue;
                }
                let message = mem::take(&mut **input_text);

                // Add to history
                history.messages.push(message);
                if history.messages.len() > history.max_messages {
                    history.messages.remove(0);
                }
            }
            Key::Space => {
                input_text.push(' ');
            }
            Key::Backspace => {
                input_text.pop();
            }
            Key::Character(character) => {
                input_text.push_str(character);
            }
            _ => continue,
        }
    }
}

fn update_chat_display(
    chat_history: Query<&ChatHistory>,
    mut history_text: Query<&mut Text2d, With<ChatHistoryText>>,
) {
    let history = chat_history.single();
    let mut display_text = history_text.single_mut();

    // Update the display with all messages
    **display_text = history.messages.join("\n");
}
