use bevy::prelude::*;
use bevy_asset::{RenderAssetUsages, UntypedAssetId};
use bevy_egui::{EguiContext, EguiContextSettings, EguiPostUpdateSet};
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use std::any::TypeId;
use bevy_render::camera::Viewport;
use bevy_render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_window::{PrimaryWindow, Window};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use fontdue::Font;

// ----- Game State and Components -----

#[derive(Resource, Default)]
struct SelectedCard {
    index: Option<usize>,
}

#[derive(Resource, Default)]
struct GameState {
    player_hand: Vec<CardData>,
    play_field: Vec<CardData>,
    player_health: u32,
    opponent_health: u32,
    current_turn: Turn,
    available_mana: u32,
}

#[derive(Default, PartialEq, Clone)]
enum Turn {
    #[default]
    Player,
    Opponent,
}

#[derive(Clone, Debug)]
struct CardData {
    name: String,
    cost: u32,
    power: u32,
    health: u32,
    description: String,
    card_type: CardType,
}

#[derive(Clone, Debug)]
enum CardType {
    Creature,
    Spell,
    Artifact,
}

// ----- Card Components -----

#[derive(Component)]
struct Card {
    index: usize,
    data: CardData,
}

#[derive(Component)]
struct CardImage;

#[derive(Component)]
struct CardText;

// ----- Hand Layout Parameters -----

#[derive(Resource, Clone, Debug)]
struct HandLayoutParams {
    count: usize,
    ideal_spacing: f32,
    spread_width: f32,
    curve_height: f32,
    base_height: f32,
    base_z: f32,
    rotation_y: f32,
    rotation_x: f32,
    z_overlap_factor: f32,
    card_curve_threshold: usize,
}

impl Default for HandLayoutParams {
    fn default() -> Self {
        Self {
            count: 3, // Default to match our initial cards
            ideal_spacing: 2.2,
            spread_width: 12.0,
            curve_height: -0.8,
            base_height: -3.0,
            base_z: 10.2,
            rotation_y: -0.3,
            rotation_x: -0.2,
            z_overlap_factor: 0.05,
            card_curve_threshold: 4,
        }
    }
}

// ----- Game Area Components -----

#[derive(Component)]
struct PlayerHandArea;

#[derive(Component)]
struct PlayFieldArea;

#[derive(Component)]
struct MainCamera;

// ----- Game UI Elements -----

#[derive(Debug)]
enum GameWindow {
    PlayingField,   // Main game view
    PlayerHand,     // Card hand
    CardCollection, // Card collection/deck building
    Inventory,      // Player inventory
    CardDetail,     // Card details/inspector
}

#[derive(Resource)]
struct UiState {
    state: DockState<GameWindow>,
    viewport_rect: egui::Rect,
    selected_entities: SelectedEntities,
    selection: GameSelection,
}

#[derive(Eq, PartialEq)]
enum GameSelection {
    CardInHand(usize),
    CardInPlay(usize),
    CardDetail(TypeId, String),
    InventoryItem(TypeId, String, UntypedAssetId),
}

// ----- Systems -----

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_plugins(bevy_egui::EguiPlugin)
        .insert_resource(UiState::new())
        .insert_resource(GameState::default())
        .init_resource::<HandLayoutParams>()
        .init_resource::<SelectedCard>()
        .add_systems(Startup, setup)
        .add_systems(
            PostUpdate,
            show_ui_system
                .before(EguiPostUpdateSet::ProcessOutput)
                .before(bevy_egui::end_pass_system)
                .before(bevy::transform::TransformSystem::TransformPropagate),
        )
        .add_systems(PostUpdate, set_camera_viewport.after(show_ui_system))
        .add_systems(Update, (
            handle_play_card,
            update_card_layout,
        ))
        .register_type::<Option<Handle<Image>>>()
        .register_type::<AlphaMode>()
        .run();
}

// Splits up the setup function into smaller parts
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut game_state: ResMut<GameState>,
    mut layout_params: ResMut<HandLayoutParams>,
) {
    setup_game_state(&mut game_state);

    // Create texture for card debugging
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    // Load font for card text
    let font_data = include_bytes!("../assets/fonts/FiraMono-Medium.ttf");
    let font = Font::from_bytes(font_data as &[u8], fontdue::FontSettings::default()).unwrap();

    setup_cards(&mut commands, &mut meshes, &mut images, &mut materials,
                &debug_material, &font, &game_state, &mut layout_params);
    setup_camera(&mut commands);
    setup_lighting(&mut commands);
    setup_play_field(&mut commands, &mut meshes, &mut materials);
}

fn setup_game_state(game_state: &mut GameState) {
    // Initialize game state with sample cards
    game_state.player_hand = vec![
        CardData {
            name: "Fire Elemental".to_string(),
            cost: 4,
            power: 5,
            health: 3,
            description: "Deals 1 damage to all enemy creatures".to_string(),
            card_type: CardType::Creature,
        },
        CardData {
            name: "Water Shield".to_string(),
            cost: 2,
            power: 0,
            health: 5,
            description: "Protects adjacent creatures".to_string(),
            card_type: CardType::Spell,
        },
        CardData {
            name: "Earth Golem".to_string(),
            cost: 6,
            power: 3,
            health: 8,
            description: "Taunt. Gains +1/+1 each turn".to_string(),
            card_type: CardType::Creature,
        },
    ];

    game_state.player_health = 30;
    game_state.opponent_health = 30;
    game_state.available_mana = 10;
}

fn setup_cards(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    images: &mut ResMut<Assets<Image>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    debug_material: &Handle<StandardMaterial>,
    font: &Font,
    game_state: &GameState,
    layout_params: &mut HandLayoutParams,
) {
    // Spawn cards for hand
    layout_params.count = game_state.player_hand.len();
    for i in 0..layout_params.count {
        spawn_card(
            commands,
            meshes,
            images,
            materials,
            debug_material,
            font,
            i,
            game_state.player_hand[i].clone(),
        );
    }
}

fn setup_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 20.0).looking_at(Vec3::new(0., 0., 10.), Vec3::Y),
        MainCamera,
    ));
}

fn setup_lighting(commands: &mut Commands) {
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

fn setup_play_field(
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
fn update_card_layout(
    game_state: Res<GameState>,
    layout_params: Res<HandLayoutParams>,
    mut card_transforms: Query<(&Card, &mut Transform)>,
) {
    // Update card positions based on hand layout
    for (card, mut transform) in card_transforms.iter_mut() {
        // For simplicity, just offset each card in hand horizontally
        let card_spacing = layout_params.spread_width / layout_params.count as f32;
        let idx = card.index as f32;
        let offset = idx * card_spacing - (layout_params.spread_width / 2.0);

        // Simple card positioning in hand
        transform.translation.x = offset;

        // In a more complex implementation, you would apply the full layout parameters
        // like the curve, rotation, etc.
    }
}

// UI Systems and Functions
fn show_ui_system(world: &mut World) {
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
fn set_camera_viewport(
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
        // Player's hand of cards - first extract the necessary data
        let cards = {
            let game_state = self.world.resource::<GameState>();
            game_state.player_hand.clone() // Clone to avoid borrow issues
        };

        let card_count = cards.len();

        // Player's hand of cards
        ui.heading("Your Hand");

        // Provide information about the cards
        for (i, card) in cards.iter().enumerate() {
            let selected = matches!(self.selection, GameSelection::CardInHand(idx) if *idx == i);

            if ui.selectable_label(selected, format!("{} ({} mana)", card.name, card.cost))
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
                    ui.heading(&card.name);
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
            ui.heading(&card.name);
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
            ui.label(&card.description);

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
            &card.name,
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

// Card creation and rendering functions
fn spawn_card(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    debug_material: &Handle<StandardMaterial>,
    font: &Font,
    index: usize,
    card_data: CardData,
) {
    // Card dimensions
    let card_size = Vec3::new(2.0, 3.0, 0.01);
    let image_size = Vec3::new(card_size.x * 0.8, card_size.y * 0.5, 0.02);
    let text_size = Vec3::new(card_size.x * 0.8, card_size.y * 0.2, 0.02);

    // Create card mesh and material
    let card_mesh = meshes.add(Cuboid::new(card_size.x, card_size.y, card_size.z));
    let image_mesh = meshes.add(Cuboid::new(image_size.x, image_size.y, image_size.z));
    let text_mesh = meshes.add(Cuboid::new(text_size.x, text_size.y, text_size.z));

    let image_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.9),
        ..default()
    });

    // Create text material for this card
    let text_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(create_text_texture(&card_data.name, font))),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            Card { index, data: card_data.clone() },
            Visibility::default(),
            Name::new(card_data.name),
        ))
        .with_children(|parent| {
            // Card base
            parent.spawn((
                Mesh3d(card_mesh.clone()),
                MeshMaterial3d(debug_material.clone()),
                Transform::default(),
            ));

            // Image section
            parent.spawn((
                Mesh3d(image_mesh.clone()),
                MeshMaterial3d(image_material.clone()),
                Transform::from_xyz(0.0, 0.1, card_size.z + image_size.z/2.0),
                CardImage,
            ));

            // Text section
            parent.spawn((
                Mesh3d(text_mesh.clone()),
                MeshMaterial3d(text_material),
                Transform::from_xyz(0.0, 1.2, card_size.z + text_size.z/2.0 + 0.005),
                CardText,
            ));
        });
}

// Texture generation functions
pub fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn create_text_texture(text: &str, font: &Font) -> Image {
    let font_size = 32.0;

    // First calculate bounds
    let mut total_width = 0.0;
    let mut max_height = 0;

    // Get metrics for all characters first
    let layout_info: Vec<_> = text
        .chars()
        .map(|ch| {
            let metrics = font.metrics(ch, font_size);
            total_width += metrics.advance_width;
            max_height = max_height.max(metrics.height as usize);
            (ch, metrics)
        })
        .collect();

    // Create texture with power of 2 dimensions
    let width = total_width.ceil() as usize;
    let height = max_height;

    // Initialize with fully transparent black
    let mut rgba = vec![0u8; width * height * 4];

    let mut x_pos = 0.0;
    for (ch, metrics) in layout_info {
        // Rasterize the character
        let (_, bitmap) = font.rasterize(ch, font_size);

        // Center character vertically
        let y_offset = (height - metrics.height) / 2;

        // Copy bitmap data into the correct position, flipping vertically
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let bitmap_idx = y * metrics.width + x;
                let alpha = bitmap[bitmap_idx];

                if alpha > 0 {
                    let tex_x = (x_pos as usize + x).min(width - 1);
                    // Flip y coordinate
                    let tex_y = height - 1 - (y_offset + y).min(height - 1);
                    let rgba_idx = (tex_y * width + tex_x) * 4;

                    // White text
                    rgba[rgba_idx] = 255;     // R
                    rgba[rgba_idx + 1] = 255; // G
                    rgba[rgba_idx + 2] = 255; // B
                    rgba[rgba_idx + 3] = alpha; // A
                }
            }
        }

        x_pos += metrics.advance_width;
    }

    Image::new_fill(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}