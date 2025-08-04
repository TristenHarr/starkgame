use bevy::prelude::*;

mod movement_trace;
mod movement_air;
mod proof_system;
mod fps_display;
mod check_constraints;

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
        .init_resource::<TraceSettings>()
        .init_resource::<ProofSystemSettings>()
        .add_systems(Startup, setup)
        .add_systems(Update, (
            (player_input, mouse_teleport_system, speed_control_system).chain(),
            update_input_state_after_modifications,
            movement_system,
            movement_trace_collection_system,
            (proof_generation_system, stats_logging_system),
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
        MovementTraceCollector::new(1.0, 5), // 1 second traces, keep 5 max
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

        if left || right || up || down {
            info!("🎮 PLAYER_INPUT: keys=({},{},{},{}) → velocity=({},{})", 
                  if left { 1 } else { 0 }, if right { 1 } else { 0 }, 
                  if up { 1 } else { 0 }, if down { 1 } else { 0 },
                  velocity.x, velocity.y);
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

        let new_state = (input_state.left, input_state.right, input_state.up, input_state.down);
        if old_state != new_state {
            info!("📝 INPUT_STATE_UPDATE: ({},{},{},{}) → ({},{},{},{}) [vel=({},{})]", 
                  if old_state.0 { 1 } else { 0 }, if old_state.1 { 1 } else { 0 }, 
                  if old_state.2 { 1 } else { 0 }, if old_state.3 { 1 } else { 0 },
                  if new_state.0 { 1 } else { 0 }, if new_state.1 { 1 } else { 0 }, 
                  if new_state.2 { 1 } else { 0 }, if new_state.3 { 1 } else { 0 },
                  velocity.x, velocity.y);
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
                    
                    info!("🔥 CHEATING: Teleported to ({:.1}, {:.1})", world_pos.x, world_pos.y);
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
            if velocity.x != 0 || velocity.y != 0 {
                info!("🔥 CHEATING: Speed hack active (3x speed)");
            }
        }
        
        if keyboard_input.pressed(KeyCode::Space) {
            // 2x speed boost
            if velocity.x != 0 || velocity.y != 0 {
                velocity.x *= 2;
                velocity.y *= 2;
                info!("🔥 SPEED_CONTROL: SPACE pressed → velocity ({},{}) → ({},{})", 
                      original_velocity.0, original_velocity.1, velocity.x, velocity.y);
            }
        }
        
        if keyboard_input.pressed(KeyCode::ControlLeft) {
            speed_multiplier = 0; // Super slow (effectively stopped)
        }
        
        if speed_multiplier != 1 {
            let pre_mult = (velocity.x, velocity.y);
            velocity.x *= speed_multiplier;
            velocity.y *= speed_multiplier;
            info!("🔥 SPEED_CONTROL: {}x multiplier → velocity ({},{}) → ({},{})", 
                  speed_multiplier, pre_mult.0, pre_mult.1, velocity.x, velocity.y);
        }
    }
}
