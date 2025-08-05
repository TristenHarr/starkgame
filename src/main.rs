use bevy::prelude::*;

mod movement_trace;
mod movement_air;
mod proof_system;
mod fps_display;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Playing,
    CheatDetected,
}

use movement_trace::*;
use proof_system::*;
use fps_display::FpsDisplayPlugin;

#[derive(Component)]
struct Player;

#[derive(Component)]
pub struct Position {
    pub x: i32, // Use integers for exact math
    pub y: i32,
}

#[derive(Component)]
pub struct Velocity {
    pub x: i32, // Use integers for exact math  
    pub y: i32,
}

#[derive(Component, Default)]
pub struct LastInputState {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

#[derive(Resource, Default)]
pub struct CheatDetected {
    pub message: String,
    pub is_active: bool,
}

#[derive(Component)]
pub struct CheatPopup;


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FpsDisplayPlugin)
        .insert_resource(bevy::winit::WinitSettings {
            focused_mode: bevy::winit::UpdateMode::reactive_low_power(
                std::time::Duration::from_nanos(16_666_667) // Exactly 60 FPS (1/60 second)
            ),
            unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(
                std::time::Duration::from_nanos(16_666_667)
            ),
        })
        .init_state::<GameState>()
        .init_resource::<ProofSystemSettings>()
        .init_resource::<CheatDetected>()
        .add_systems(Startup, setup)
        .add_systems(Update, (
            // Input systems only run in Playing state
            (player_input, mouse_teleport_system, speed_control_system).chain().run_if(in_state(GameState::Playing)),
            update_input_state_after_modifications.run_if(in_state(GameState::Playing)),
            // CRITICAL: Movement system ONLY runs in Playing state - no position updates during cheat state
            movement_system.run_if(in_state(GameState::Playing)),
            // CRITICAL: Trace collection ONLY runs in Playing state - stops immediately when cheat detected
            movement_trace_collection_system.run_if(in_state(GameState::Playing)),
            // CRITICAL: Proof generation ONLY runs in Playing state - no proofs generated during cheat state
            (proof_generation_system, stats_logging_system).run_if(in_state(GameState::Playing)),
            cheat_detection_system,
            cheat_popup_system.run_if(in_state(GameState::CheatDetected)),
            dismiss_cheat_popup_system.run_if(in_state(GameState::CheatDetected)),
        ).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.7, 0.9),
            custom_size: Some(Vec2::new(50.0, 50.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Player,
        Position { x: 0, y: 0 },
        Velocity { x: 0, y: 0 },
        LastInputState::default(),
        MovementTraceCollector::new(0.1, 5), // 0.1 second traces, keep 5 max
        ProofGenerator::default(),
    ));
}

fn player_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    for mut velocity in &mut query {
        let left = keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA);
        let right = keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD);
        let up = keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW);
        let down = keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS);

        velocity.x = 0;
        velocity.y = 0;

        if left {
            velocity.x = -200;
        }
        if right {
            velocity.x = 200;
        }
        if up {
            velocity.y = 200;
        }
        if down {
            velocity.y = -200;
        }

    }
}

// Capture the input state that matches the ACTUAL game velocity logic
fn update_input_state_after_modifications(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Velocity, &mut LastInputState), With<Player>>,
) {
    for (velocity, mut input_state) in &mut query {
        let old_state = (input_state.left, input_state.right, input_state.up, input_state.down);
        
        // Match the EXACT same logic as player_input system
        // This ensures perfect synchronization with the actual velocity
        input_state.left = false;
        input_state.right = false;
        input_state.up = false;
        input_state.down = false;

        // X-axis: right wins over left (same as player_input logic)
        if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
            input_state.left = true;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
            input_state.right = true;
            input_state.left = false; // Right overrides left
        }

        // Y-axis: Check the actual player_input logic order
        if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
            input_state.up = true;
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
            input_state.down = true;
            input_state.up = false; // Down overrides up (matches player_input order)
        }

    }
}

fn movement_system(
    mut query: Query<(&mut Transform, &mut Position, &Velocity)>,
) {
    // Completely deterministic integer math that works identically in debug/release
    for (mut transform, mut position, velocity) in &mut query {
        // Completely avoid division - use only multiplication and addition
        // Since constraint expects: position_change = velocity * 15
        // We need: position += velocity * 15 / 1000, but avoiding division
        // So: position += (velocity * 15) / 1000
        // For velocity 200: 200 * 15 = 3000, 3000 / 1000 = 3
        let delta_x = (velocity.x * 15) / 1000;
        let delta_y = (velocity.y * 15) / 1000;
        position.x += delta_x;
        position.y += delta_y;
        
        // Convert to float for rendering only
        transform.translation.x = position.x as f32;
        transform.translation.y = position.y as f32;
    }
}

// Cheating system: teleport to mouse click position
fn mouse_teleport_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut player_query: Query<(&mut Transform, &mut Position), With<Player>>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        let window = windows.single();
        let (camera, camera_transform) = camera_q.single();
        
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) {
                let world_pos = ray.origin.truncate(); // Convert Ray3d to Vec2
                for (mut transform, mut position) in &mut player_query {
                    // This is cheating! Instant teleportation should violate proof constraints
                    transform.translation.x = world_pos.x;
                    transform.translation.y = world_pos.y;
                    position.x = world_pos.x as i32;
                    position.y = world_pos.y as i32;
                }
            }
        }
    }
}

// Cheating system: modify movement speed
fn speed_control_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    for mut velocity in &mut query {
        let original_velocity = (velocity.x, velocity.y);
        let mut speed_multiplier = 1;
        
        if keyboard_input.pressed(KeyCode::ShiftLeft) {
            speed_multiplier = 3; // Speed hack!
        }
        
        if keyboard_input.pressed(KeyCode::Space) {
            // 2x speed boost
            if velocity.x != 0 || velocity.y != 0 {
                velocity.x *= 2;
                velocity.y *= 2;
            }
        }
        
        if keyboard_input.pressed(KeyCode::ControlLeft) {
            speed_multiplier = 0; // Super slow (effectively stopped)
        }
        
        if speed_multiplier != 1 {
            let pre_mult = (velocity.x, velocity.y);
            velocity.x *= speed_multiplier;
            velocity.y *= speed_multiplier;
        }
    }
}

// System to detect cheating from proof verification failures
fn cheat_detection_system(
    mut player_query: Query<(&mut MovementTraceCollector, &ProofGenerator), With<Player>>,
    mut cheat_detected: ResMut<CheatDetected>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
) {
    for (mut trace_collector, proof_gen) in &mut player_query {
        // Simple detection: any failures indicate cheating
        if proof_gen.stats.failed_verifications > 0 && !cheat_detected.is_active && *current_state.get() == GameState::Playing {
            cheat_detected.is_active = true;
            cheat_detected.message = "CHEATER DETECTED!\nInvalid proof verification failed!\nPress ESC to continue".to_string();
            next_state.set(GameState::CheatDetected);
            
            // CRITICAL: Immediately terminate and clear all active traces when cheat detected
            trace_collector.current_trace = None;
            trace_collector.completed_traces.clear();
        }
    }
}

// System to show the cheat popup
fn cheat_popup_system(
    mut commands: Commands,
    cheat_detected: Res<CheatDetected>,
    existing_popup: Query<Entity, With<CheatPopup>>,
) {
    if cheat_detected.is_active && existing_popup.is_empty() {
        // Create the popup UI
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(25.0),
                top: Val::Percent(25.0),
                width: Val::Percent(50.0),
                height: Val::Percent(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.8, 0.1, 0.1, 0.95)), // Red background
            CheatPopup,
        )).with_children(|parent| {
            parent.spawn((
                Text::new(&cheat_detected.message),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    align_self: AlignSelf::Center,
                    ..default()
                },
            ));
        });
    }
}

// System to handle ESC key to dismiss the popup and reset game state
fn dismiss_cheat_popup_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut cheat_detected: ResMut<CheatDetected>,
    mut commands: Commands,
    popup_query: Query<Entity, With<CheatPopup>>,
    mut player_query: Query<(&mut Transform, &mut Position, &mut Velocity, &mut MovementTraceCollector, &mut ProofGenerator), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) && cheat_detected.is_active {
        // Clear cheat state
        cheat_detected.is_active = false;
        cheat_detected.message.clear();
    
        // Remove popup UI
        for entity in &popup_query {
            commands.entity(entity).despawn_recursive();
        }
        
        // Reset game state
        for (mut transform, mut position, mut velocity, mut trace_collector, mut proof_gen) in &mut player_query {
            // Reset player to starting position
            transform.translation = Vec3::new(0.0, 0.0, 0.0);
            position.x = 0;
            position.y = 0;
            velocity.x = 0;
            velocity.y = 0;
            
            // Clear trace and proof history
            trace_collector.completed_traces.clear();
            trace_collector.current_trace = None;
            // CRITICAL: Mark next trace as first after reset to enforce origin constraint
            trace_collector.mark_next_trace_as_first_after_reset();
            
            // Reset proof generator stats
            proof_gen.active_tasks.clear();
            proof_gen.completed_count = 0;
            proof_gen.stats = ProofStats::default();
        }
        
        // CRITICAL: Reset failed_verifications BEFORE transitioning back to Playing state
        // This prevents the same cheat from being processed again in the next frame
        for (_, _, _, _, mut proof_gen) in &mut player_query {
            proof_gen.stats.failed_verifications = 0;
        }
        
        // CRITICAL: Transition back to Playing state happens NEXT frame
        // This ensures input systems cannot run in the same frame as the reset
        next_state.set(GameState::Playing);
    }
}
