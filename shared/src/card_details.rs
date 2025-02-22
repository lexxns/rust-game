use serde::Deserialize;
use std::collections::HashMap;
use crate::EntityID;

#[derive(Debug, Deserialize)]
pub struct CardDefinition {
    pub name: String,
    pub text: String,
    pub c_type: String,
    pub cost: u32,
    pub power: u32,
}

#[derive(Debug, Deserialize)]
pub struct CardConfig {
    pub cards: HashMap<String, CardDefinition>,
}

pub fn load_cards() -> Result<CardConfig, Box<dyn std::error::Error>> {
    let config_str = include_str!("../assets/cards.toml");
    let config: CardConfig = toml::from_str(config_str)?;
    Ok(config)
}

pub fn build_default_deck() -> Vec<(EntityID, String, String)> {
    let config = load_cards().expect("Failed to load card definitions");
    let mut deck = Vec::new();
    let mut card_id = 0;

    // Add two of each card to the deck
    for (_, card_def) in config.cards.iter() {
        for _ in 0..2 {
            deck.push((
                card_id,
                card_def.name.clone(),
                card_def.text.clone(),
            ));
            card_id += 1;
        }
    }

    deck
}