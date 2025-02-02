use std::cmp::PartialEq;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use fontdue::Font;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bevy::asset::{ron, AssetLoader};
use bevy::asset::io::AssetSourceEvent;
use bevy::asset::io::file::FileWatcher;
use serde::Deserialize;
use crossbeam_channel::{Receiver};


fn main() {

    // Get absolute path to assets directory
    let mut asset_path = env::current_dir().expect("Failed to get current directory");
    asset_path.push("experiments/assets");
    println!("Asset directory: {:?}", asset_path);

    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            #[cfg(not(target_arch = "wasm32"))]
            WireframePlugin,
        ))
        .insert_resource(HandLayoutParams::default())
        .insert_resource(AssetDirectory(asset_path.clone()))
        .add_systems(Startup, (setup, setup_file_watcher))
        .add_systems(
            Update,
            (
                #[cfg(not(target_arch = "wasm32"))]
                toggle_wireframe,
                update_card_count,
                update_card_positions,
                handle_asset_events
            ),
        )
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

const SHAPES_X_EXTENT: f32 = 14.0;
const EXTRUSION_X_EXTENT: f32 = 16.0;
const Z_EXTENT: f32 = 5.0;

// Component to mark our card entities
#[derive(Component)]
struct Card {
    index: usize,
}

// Component for the card's image section
#[derive(Component)]
struct CardImage;


// Component for the card's text section
#[derive(Component)]
struct CardText;

#[derive(Resource, Deserialize, Clone, Debug)]
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
            count: 5,
            ideal_spacing: 2.0,
            spread_width: 12.0,
            curve_height: -1.0,
            base_height: -3.0,
            base_z: 10.0,
            rotation_y: -0.5,
            rotation_x: -0.2,
            z_overlap_factor: 0.1,
            card_curve_threshold: 5,
        }
    }
}

fn create_text_texture(text: &str, font: &Font) -> Image {
    let font_size = 32.0;

    // First calculate bounds
    let mut total_width = 0.0;
    let mut max_height = 0;

    // Get metrics for all characters first
    let layout_info: Vec<_> = text.chars().map(|ch| {
        let metrics = font.metrics(ch, font_size);
        total_width += metrics.advance_width;
        max_height = max_height.max(metrics.height as usize);
        (ch, metrics)
    }).collect();

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

fn spawn_card(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    debug_material: &Handle<StandardMaterial>,
    font: &Font,
    index: usize,
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
        base_color_texture: Some(images.add(create_text_texture(&format!("Card {}", index + 1), font))),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            Card { index },
            Visibility::default(),
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut layout_params: ResMut<HandLayoutParams>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    let font_data = include_bytes!("../assets/fonts/FiraMono-Medium.ttf");
    let font = Font::from_bytes(font_data as &[u8], fontdue::FontSettings::default()).unwrap();

    // Spawn initial cards
    for i in 0..layout_params.count {
        spawn_card(
            &mut commands,
            &mut meshes,
            &mut images,
            &mut materials,
            &debug_material,
            &font,
            i,
        );
    }

    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // // ground plane
    // commands.spawn((
    //     Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
    //     MeshMaterial3d(materials.add(Color::from(SILVER))),
    // ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 20.0).looking_at(Vec3::new(0., 0., 10.), Vec3::Y),
    ));

    #[cfg(not(target_arch = "wasm32"))]
    commands.spawn((
        Text::new("Press space to toggle wireframes"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

// makes all cards rotate in place
fn rotate(mut query: Query<&mut Transform, With<Card>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

#[derive(Resource)]
struct AssetDirectory(PathBuf);

#[derive(Resource)]
struct AssetWatcher(FileWatcher);

#[derive(Resource)]
struct AssetEventReceiver(Receiver<AssetSourceEvent>);

fn setup_file_watcher(mut commands: Commands) {
    let (tx, rx) = crossbeam_channel::bounded(16);

    use std::env;

    // Get the current executable's directory
    let mut asset_path = env::current_dir().expect("Failed to get current directory");
    asset_path.push("experiments/assets");

    println!("Watching asset directory: {:?}", asset_path);

    let watcher = FileWatcher::new(
        asset_path,
        tx,
        Duration::from_millis(200),
    ).expect("Failed to create file watcher");

    commands.insert_resource(AssetWatcher(watcher));
    commands.insert_resource(AssetEventReceiver(rx));
}

impl PartialEq for HandLayoutParams {
    fn eq(&self, other: &Self) -> bool {
        const EPSILON: f32 = 0.0001;  // Small threshold for floating point comparison

        self.count == other.count &&
        (self.spread_width - other.spread_width).abs() < EPSILON &&
        (self.curve_height - other.curve_height).abs() < EPSILON &&
        (self.base_height - other.base_height).abs() < EPSILON &&
        (self.base_z - other.base_z).abs() < EPSILON &&
        (self.rotation_x - other.rotation_x).abs() < EPSILON &&
        (self.rotation_y - other.rotation_y).abs() < EPSILON
    }
}

fn handle_asset_events(
    receiver: Res<AssetEventReceiver>,
    asset_dir: Res<AssetDirectory>,
    mut layout_params: ResMut<HandLayoutParams>,
) {
    while let Ok(event) = receiver.0.try_recv() {
        match event {
            AssetSourceEvent::ModifiedAsset(rel_path) => {
                if rel_path.extension().map_or(false, |ext| ext == "ron") {
                    let full_path = asset_dir.0.join(&rel_path);
                    println!("Reading config from: {:?}", full_path);

                    match std::fs::read_to_string(&full_path) {
                        Ok(content) => {
                            println!("File contents:\n{}", content);
                            match ron::from_str::<HandLayoutParams>(&content) {
                                Ok(new_params) => {
                                    if new_params != *layout_params {
                                        println!("Updating params from:\n{:?}\nto:\n{:?}",
                                                 *layout_params, new_params);
                                        *layout_params = new_params;
                                        layout_params.set_changed();
                                    }
                                },
                                Err(e) => println!("Failed to parse RON: {}", e),
                            }
                        },
                        Err(e) => println!("Failed to read file: {}", e),
                    }
                }
            }
            _ => {}
        }
    }
}

// System to handle card count changes
fn update_card_count(
    mut commands: Commands,
    params: Res<HandLayoutParams>,
    card_query: Query<Entity, With<Card>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if params.is_changed() {
        let current_count = card_query.iter().count();

        if current_count != params.count {
            // Despawn all existing cards
            for entity in card_query.iter() {
                commands.entity(entity).despawn_recursive();
            }

            // Create shared resources
            let debug_material = materials.add(StandardMaterial {
                base_color_texture: Some(images.add(uv_debug_texture())),
                ..default()
            });

            let font_data = include_bytes!("../assets/fonts/FiraMono-Medium.ttf");
            let font = Font::from_bytes(font_data as &[u8], fontdue::FontSettings::default()).unwrap();

            // Spawn new cards
            for i in 0..params.count {
                spawn_card(
                    &mut commands,
                    &mut meshes,
                    &mut images,
                    &mut materials,
                    &debug_material,
                    &font,
                    i,
                );
            }
        }
    }
}

fn update_card_positions(
    params: Res<HandLayoutParams>,
    mut query: Query<(&Card, &mut Transform)>,
) {
    if params.is_changed() {
        println!("Applying new card positions with params: {:?}", *params);
    }

    let desired_total_width = if params.count <= 1 {
        0.0
    } else {
        params.ideal_spacing * (params.count as f32 - 1.0)
    };

    let actual_total_width = desired_total_width.min(params.spread_width);
    let card_spacing = if params.count <= 1 {
        0.0
    } else {
        actual_total_width / (params.count as f32 - 1.0)
    };

    let z_overlap_factor = 0.1;

    for (card, mut transform) in query.iter_mut() {
        let i = card.index as f32;

        let x = if params.count <= 1 {
            0.0
        } else {
            let half_width = (actual_total_width / 2.0);
            let start_x = -half_width;
            start_x + (i * card_spacing)
        };

        let normalized_x = (x / (params.spread_width/2.0)).abs();

        // Only apply curve if we're over the threshold
        let y = if params.count > params.card_curve_threshold {
            params.base_height + (normalized_x * normalized_x * params.curve_height)
        } else {
            params.base_height
        };

        let z = if params.count > params.card_curve_threshold {
            params.base_z - normalized_x * 0.5 + (x.signum() * z_overlap_factor)
        } else {
            params.base_z
        };

        transform.translation = Vec3::new(x, y, z);

        // Similarly, only apply y-rotation if we're over the threshold
        let rotation = if params.count > params.card_curve_threshold {
            Quat::from_rotation_y(normalized_x * params.rotation_y * x.signum())
                * Quat::from_rotation_x(params.rotation_x)
        } else {
            // Just apply x rotation when not curving
            Quat::from_rotation_x(params.rotation_x)
        };

        transform.rotation = rotation;
    }
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
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

#[cfg(not(target_arch = "wasm32"))]
fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        wireframe_config.global = !wireframe_config.global;
    }
}