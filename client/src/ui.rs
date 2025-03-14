use bevy::prelude::*;
use bevy::asset::Assets;
use bevy::color::Color;
use bevy::math::UVec2;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::render::camera::Viewport;
use bevy_inspector_egui::bevy_egui::{EguiContext, EguiContextSettings};
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;
use bevy_inspector_egui::egui;
use crate::state::{UiState, GameState, GameWindow, GameSelection, Turn, SelectedCard};
use bevy_window::{PrimaryWindow, Window};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use shared::channel::{CardData, CardType};

#[derive(Component)]
pub(crate) struct PlayerHandArea;

#[derive(Component)]
pub(crate) struct PlayFieldArea;

#[derive(Component)]
pub(crate) struct MainCamera;

pub(crate) fn setup_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 20.0).looking_at(Vec3::new(0., 0., 10.), Vec3::Y),
        MainCamera,
    ));
}

pub(crate) fn setup_lighting(commands: &mut Commands) {
    commands.spawn((
        PointLight {
            intensity: 10_000_000.,
            shadows_enabled: true,
            range: 100.0,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));
}

pub(crate) fn setup_play_field(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>
) {
    // Add a simple playing field in the center
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(15.0, 15.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::rgb(0.1, 0.5, 0.1),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        PlayFieldArea,
    ));
}

// New system to handle playing a card
fn handle_play_card(
    mut game_state: ResMut<GameState>,
    mut selected_card: ResMut<SelectedCard>,
    mut commands: Commands,
    // Add other parameters needed for your gameplay logic
) {
    // This would normally react to events or other triggers
    // For example, could check for card played events here

    // Handle moving cards from hand to play field
    if let Some(idx) = selected_card.index {
        // In a real implementation, you would check if the card was actually played
        // and move it from hand to play field

        // For example code purposes only:
        // if card_was_played {
        //     let card = game_state.player_hand.remove(idx);
        //     game_state.play_field.push(card);
        //     game_state.available_mana -= card.cost;
        //     selected_card.index = None;
        // }
    }
}

// New system to update card layout in hand
// fn update_card_layout(
//     game_state: Res<GameState>,
//     layout_params: Res<HandLayoutParams>,
//     mut card_transforms: Query<(&CardData, &mut Transform)>,
// ) {
//     // Update card positions based on hand layout
//     for (card, mut transform) in card_transforms.iter_mut() {
//         // For simplicity, just offset each card in hand horizontally
//         let card_spacing = layout_params.spread_width / layout_params.count as f32;
//         let idx = card.index();
//         let offset = idx * card_spacing - (layout_params.spread_width / 2.0);
//
//         // Simple card positioning in hand
//         transform.translation.x = offset;
//
//         // In a more complex implementation, you would apply the full layout parameters
//         // like the curve, rotation, etc.
//     }
// }

// UI Systems and Functions
pub(crate) fn show_ui_system(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    world.resource_scope::<UiState, _>(|world, mut ui_state| {
        ui_state.ui(world, egui_context.get_mut())
    });
}

// Camera system
pub(crate) fn set_camera_viewport(
    ui_state: Res<UiState>,
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    egui_settings: Query<&EguiContextSettings>,
    mut cameras: Query<&mut Camera, With<MainCamera>>,
) {
    let mut cam = cameras.single_mut();

    let Ok(window) = primary_window.get_single() else {
        return;
    };

    let scale_factor = window.scale_factor() * egui_settings.single().scale_factor;

    let viewport_pos = ui_state.viewport_rect.left_top().to_vec2() * scale_factor;
    let viewport_size = ui_state.viewport_rect.size() * scale_factor;

    let physical_position = UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32);
    let physical_size = UVec2::new(viewport_size.x as u32, viewport_size.y as u32);

    // Check if viewport is valid
    let rect = physical_position + physical_size;
    let window_size = window.physical_size();

    if rect.x <= window_size.x && rect.y <= window_size.y {
        cam.viewport = Some(Viewport {
            physical_position,
            physical_size,
            depth: 0.0..1.0,
        });
    }
}

// UI State implementation
impl UiState {
    pub fn new() -> Self {
        let mut state = DockState::new(vec![GameWindow::PlayingField]);
        let tree = state.main_surface_mut();

        // Layout for game elements
        let [game, _card_detail] =
            tree.split_right(NodeIndex::root(), 0.75, vec![GameWindow::CardDetail]);
        let [game, _player_hand] = tree.split_left(game, 0.2, vec![GameWindow::PlayerHand]);
        let [_game, _bottom] =
            tree.split_below(game, 0.8, vec![GameWindow::CardCollection, GameWindow::Inventory]);

        Self {
            state,
            selected_entities: SelectedEntities::default(),
            selection: GameSelection::CardInHand(0),
            viewport_rect: egui::Rect::NOTHING,
        }
    }

    fn ui(&mut self, world: &mut World, ctx: &mut egui::Context) {
        let mut tab_viewer = GameTabViewer {
            world,
            viewport_rect: &mut self.viewport_rect,
            selected_entities: &mut self.selected_entities,
            selection: &mut self.selection,
        };
        DockArea::new(&mut self.state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);
    }
}

// Tab viewer for the UI
struct GameTabViewer<'a> {
    world: &'a mut World,
    selected_entities: &'a mut SelectedEntities,
    selection: &'a mut GameSelection,
    viewport_rect: &'a mut egui::Rect,
}

impl egui_dock::TabViewer for GameTabViewer<'_> {
    type Tab = GameWindow;

    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, window: &mut Self::Tab) {
        match window {
            GameWindow::PlayingField => self.render_playing_field(ui),
            GameWindow::PlayerHand => self.render_player_hand(ui),
            GameWindow::CardCollection => self.render_card_collection(ui),
            GameWindow::Inventory => self.render_inventory(ui),
            GameWindow::CardDetail => self.render_card_detail(ui),
        }
    }

    fn title(&mut self, window: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        match window {
            GameWindow::PlayingField => "Playing Field".into(),
            GameWindow::PlayerHand => "Your Hand".into(),
            GameWindow::CardCollection => "Card Collection".into(),
            GameWindow::Inventory => "Inventory".into(),
            GameWindow::CardDetail => "Card Details".into(),
        }
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        !matches!(window, GameWindow::PlayingField)
    }
}

// Breaking up the UI rendering into separate methods
impl GameTabViewer<'_> {
    fn render_playing_field(&mut self, ui: &mut egui_dock::egui::Ui) {
        // Main game view with the playing field
        *self.viewport_rect = ui.clip_rect();

        // Get game state data for this panel
        let (player_health, opponent_health, available_mana, current_turn) = {
            let game_state = self.world.resource::<GameState>();
            (
                game_state.player_health,
                game_state.opponent_health,
                game_state.available_mana,
                game_state.current_turn.clone(),
            )
        };

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Player Health: {}", player_health));
                ui.label(format!("Mana: {}/10", available_mana));
                ui.label(format!("Opponent Health: {}", opponent_health));
            });
            ui.label(format!(
                "Turn: {}",
                if current_turn == Turn::Player {
                    "Your Turn"
                } else {
                    "Opponent Turn"
                }
            ));
        });
    }

    fn render_player_hand(&mut self, ui: &mut egui_dock::egui::Ui) {
        let cards = {
            let game_state = self.world.resource::<GameState>();
            game_state.player_hand.clone()
        };

        let card_count = cards.len();

        // Player's hand of cards
        ui.heading("Your Hand");

        // Provide information about the cards
        for (i, card) in cards.iter().enumerate() {
            let selected = matches!(self.selection, GameSelection::CardInHand(idx) if *idx == i);

            if ui.selectable_label(selected, format!("{} ({} mana)", card.card_name, card.cost))
                .clicked()
            {
                *self.selection = GameSelection::CardInHand(i);

                // Set selected card
                let mut selected_card = self.world.resource_mut::<SelectedCard>();
                selected_card.index = Some(i);
            }
        }

        ui.separator();
        ui.label(format!("Cards in hand: {}", card_count));

        // Draw card button
        if ui.button("Draw Card").clicked() {
            // In a complete game, this would add a card to the hand
            ui.label("Drew a card! (simulated)");
        }
    }

    fn render_card_collection(&mut self, ui: &mut egui_dock::egui::Ui) {
        // Player's card collection/deck building area
        ui.heading("Card Collection");
        ui.label("Build your deck by selecting cards from your collection:");

        // Example card categories
        ui.collapsing("Creatures", |ui| {
            ui.label("Fire Elemental (4 mana)");
            ui.label("Water Spirit (3 mana)");
            ui.label("Earth Golem (6 mana)");
            ui.label("Air Wisp (2 mana)");
        });

        ui.collapsing("Spells", |ui| {
            ui.label("Fireball (3 mana)");
            ui.label("Healing Rain (4 mana)");
            ui.label("Stone Wall (2 mana)");
            ui.label("Lightning Bolt (1 mana)");
        });

        ui.collapsing("Artifacts", |ui| {
            ui.label("Mana Crystal (0 mana)");
            ui.label("Ancient Tome (2 mana)");
            ui.label("Enchanted Armor (3 mana)");
        });
    }

    fn render_inventory(&mut self, ui: &mut egui_dock::egui::Ui) {
        // Player's inventory
        ui.heading("Inventory");

        ui.collapsing("Resources", |ui| {
            ui.label("Gold: 1250");
            ui.label("Dust: 350");
            ui.label("Card Packs: 3");
        });

        ui.collapsing("Achievements", |ui| {
            ui.label("✓ Win your first game");
            ui.label("✓ Build a custom deck");
            ui.label("✗ Win with only spells");
            ui.label("✗ Collect all rare cards");
        });

        ui.collapsing("Game Stats", |ui| {
            ui.label("Games played: 15");
            ui.label("Wins: 8");
            ui.label("Losses: 7");
            ui.label("Win rate: 53%");
        });
    }

    fn render_card_detail(&mut self, ui: &mut egui_dock::egui::Ui) {
        match *self.selection {
            GameSelection::CardInHand(idx) => {
                self.render_hand_card_detail(ui, idx);
            }
            GameSelection::CardInPlay(idx) => {
                // Get play field card data
                let card_data = {
                    let game_state = self.world.resource::<GameState>();
                    if idx < game_state.play_field.len() {
                        Some(game_state.play_field[idx].clone())
                    } else {
                        None
                    }
                };

                if let Some(card) = card_data {
                    ui.heading(&card.card_name);
                    ui.label("Card in play");
                    // More card details would go here
                }
            }
            GameSelection::CardDetail(_, ref name) => {
                ui.label(format!("Card Detail: {}", name));
            }
            GameSelection::InventoryItem(_, ref name, _) => {
                ui.label(format!("Inventory: {}", name));
            }
        }
    }

    fn render_hand_card_detail(&mut self, ui: &mut egui_dock::egui::Ui, idx: usize) {
        // Create a local copy of the card data we need to avoid the borrow conflict
        let (card, can_play) = {
            let game_state = self.world.resource::<GameState>();
            if idx < game_state.player_hand.len() {
                let card = game_state.player_hand[idx].clone(); // Clone to end the borrow
                let can_play = card.cost <= game_state.available_mana;
                (Some(card), can_play)
            } else {
                (None, false)
            }
        }; // game_state borrow ends here

        if let Some(card) = card {
            ui.heading(&card.card_name);
            ui.horizontal(|ui| {
                ui.label(format!("Cost: {} mana", card.cost));
                match card.card_type {
                    CardType::Creature => {
                        ui.label(format!("Power: {}", card.power));
                        ui.label(format!("Health: {}", card.health));
                    },
                    CardType::Spell => {
                        ui.label("Type: Spell");
                    },
                    CardType::Artifact => {
                        ui.label("Type: Artifact");
                    }
                }
            });

            ui.separator();
            ui.label(&card.card_text);

            ui.separator();
            // Play card button
            if ui.button("Play Card").clicked() && can_play {
                // Now we can mutably borrow since the immutable borrow is dropped
                self.world.resource_scope::<SelectedCard, _>(|_, mut selected_card| {
                    selected_card.index = None;
                });

                // Trigger gameplay actions
                // In a real implementation, we would trigger an event here to be handled by a gameplay system
                ui.label("Card played! (simulated)");
            }

            self.render_card_preview(ui, &card);
        } else {
            ui.label("No card selected");
        }
    }

    fn render_card_preview(&mut self, ui: &mut egui_dock::egui::Ui, card: &CardData) {
        // Card preview visualization
        let card_image_size = [120.0, 180.0];
        let (rect, _) = ui.allocate_exact_size(card_image_size.into(), egui::Sense::hover());

        // Draw a simplified card preview
        let card_color = match card.card_type {
            CardType::Creature => egui::Color32::from_rgb(200, 100, 100),
            CardType::Spell => egui::Color32::from_rgb(100, 100, 200),
            CardType::Artifact => egui::Color32::from_rgb(100, 200, 100),
        };

        ui.painter().rect_filled(rect, 5.0, card_color);

        // Draw card name in the preview
        let text_pos = rect.min + egui::vec2(10.0, 20.0);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_TOP,
            &card.card_name,
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );

        // Draw card stats
        if let CardType::Creature = card.card_type {
            // Draw power/health at bottom right
            let stats_pos = rect.max - egui::vec2(20.0, 20.0);
            ui.painter().text(
                stats_pos,
                egui::Align2::RIGHT_BOTTOM,
                format!("{}/{}", card.power, card.health),
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            );
        }

        // Draw mana cost at top right
        let mana_pos = rect.min + egui::vec2(rect.width() - 20.0, 20.0);
        ui.painter().circle_filled(
            mana_pos,
            15.0,
            egui::Color32::from_rgb(0, 100, 200),
        );
        ui.painter().text(
            mana_pos,
            egui::Align2::CENTER_CENTER,
            format!("{}", card.cost),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );
    }
}