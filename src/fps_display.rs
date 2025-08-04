use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::winit::WinitSettings;
use crate::{Player, ProofGenerator, Velocity};

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct ProofStatsText;

#[derive(Component)]
struct FpsLimitText;

#[derive(Component)]
struct VelocityText;

#[derive(Resource)]
struct FpsControls {
    current_limit: Option<f64>, // None = unlimited
    available_limits: Vec<Option<f64>>,
    current_index: usize,
}

impl Default for FpsControls {
    fn default() -> Self {
        Self {
            current_limit: None, // Start with unlimited
            available_limits: vec![
                None,         // Unlimited
                Some(240.0),  // Very high
                Some(144.0),  // High refresh
                Some(120.0),  // Gaming
                Some(60.0),   // Standard
                Some(30.0),   // Lower
            ],
            current_index: 0,
        }
    }
}

pub struct FpsDisplayPlugin;

impl Plugin for FpsDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(WinitSettings {
                focused_mode: bevy::winit::UpdateMode::Continuous,
                unfocused_mode: bevy::winit::UpdateMode::Continuous,
            })
            .init_resource::<FpsControls>()
            .add_systems(Startup, setup_fps_display)
            .add_systems(Update, (
                update_fps_display, 
                update_proof_stats_display,
                handle_fps_controls,
                update_fps_limit_display,
                update_velocity_display,
            ));
    }
}

fn setup_fps_display(mut commands: Commands) {
    // Create UI root
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Container for all stats
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    ..default()
                })
                .with_children(|stats_parent| {
                    // FPS text
                    stats_parent.spawn((
                        Text::new("FPS: --"),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        FpsText,
                    ));

                    // Proof stats text
                    stats_parent.spawn((
                        Text::new("Proofs: Active: 0, Generated: 0"),
                        TextColor(Color::srgb(0.8, 0.8, 1.0)),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        ProofStatsText,
                    ));

                    // FPS limit control text
                    stats_parent.spawn((
                        Text::new("FPS Limit: Unlimited (Press F to cycle)"),
                        TextColor(Color::srgb(0.8, 1.0, 0.8)),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        FpsLimitText,
                    ));
                    
                    // Velocity display text
                    stats_parent.spawn((
                        Text::new("Velocity: (0.0, 0.0) - Press SPACE to speed hack!"),
                        TextColor(Color::srgb(1.0, 0.8, 0.8)),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        VelocityText,
                    ));
                });
        });
}

fn update_fps_display(
    diagnostics: Res<DiagnosticsStore>,
    mut text_query: Query<&mut Text, With<FpsText>>,
    mut color_query: Query<&mut TextColor, With<FpsText>>,
) {
    if let (Ok(mut text), Ok(mut text_color)) = (text_query.get_single_mut(), color_query.get_single_mut()) {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                // Color-code the FPS based on performance
                let color = if value >= 55.0 {
                    Color::srgb(0.0, 1.0, 0.0) // Green for good FPS
                } else if value >= 30.0 {
                    Color::srgb(1.0, 1.0, 0.0) // Yellow for okay FPS
                } else {
                    Color::srgb(1.0, 0.0, 0.0) // Red for poor FPS
                };
                
                **text = format!("FPS: {:.1}", value);
                text_color.0 = color;
            }
        }
    }
}

fn update_proof_stats_display(
    mut text_query: Query<&mut Text, With<ProofStatsText>>,
    mut color_query: Query<&mut TextColor, With<ProofStatsText>>,
    proof_query: Query<&ProofGenerator, With<Player>>,
) {
    if let (Ok(mut text), Ok(mut text_color)) = (text_query.get_single_mut(), color_query.get_single_mut()) {
        if let Ok(proof_gen) = proof_query.get_single() {
            let active_count = proof_gen.active_tasks.len();
            let generated_count = proof_gen.stats.total_proofs_generated;
            let avg_time = proof_gen.stats.avg_generation_time();
            
            **text = format!(
                "Proofs: Active: {}, Generated: {}, Avg: {:.1}ms", 
                active_count, generated_count, avg_time
            );
            
            // Color-code based on activity
            text_color.0 = if active_count > 0 {
                Color::srgb(1.0, 0.8, 0.0) // Orange when actively generating
            } else {
                Color::srgb(0.8, 0.8, 1.0) // Light blue when idle
            };
        }
    }
}

fn handle_fps_controls(
    mut fps_controls: ResMut<FpsControls>,
    mut winit_settings: ResMut<WinitSettings>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyF) {
        // Cycle to next FPS limit
        fps_controls.current_index = (fps_controls.current_index + 1) % fps_controls.available_limits.len();
        fps_controls.current_limit = fps_controls.available_limits[fps_controls.current_index];
        
        // Update Bevy's settings based on the limit
        match fps_controls.current_limit {
            None => {
                // Unlimited FPS
                winit_settings.focused_mode = bevy::winit::UpdateMode::Continuous;
                winit_settings.unfocused_mode = bevy::winit::UpdateMode::Continuous;
            },
            Some(fps) => {
                // Limited FPS
                let target_frametime = std::time::Duration::from_secs_f64(1.0 / fps);
                winit_settings.focused_mode = bevy::winit::UpdateMode::reactive_low_power(target_frametime);
                winit_settings.unfocused_mode = bevy::winit::UpdateMode::reactive_low_power(target_frametime);
            }
        }
        
        info!("FPS limit changed to: {:?}", fps_controls.current_limit);
    }
}

fn update_fps_limit_display(
    fps_controls: Res<FpsControls>,
    mut text_query: Query<&mut Text, With<FpsLimitText>>,
) {
    if let Ok(mut text) = text_query.get_single_mut() {
        let limit_text = match fps_controls.current_limit {
            None => "Unlimited".to_string(),
            Some(fps) => format!("{:.0}", fps),
        };
        
        **text = format!("FPS Limit: {} (Press F to cycle)", limit_text);
    }
}

fn update_velocity_display(
    mut text_query: Query<&mut Text, With<VelocityText>>,
    mut color_query: Query<&mut TextColor, With<VelocityText>>,
    player_query: Query<&Velocity, With<Player>>,
) {
    if let (Ok(mut text), Ok(mut text_color)) = (text_query.get_single_mut(), color_query.get_single_mut()) {
        if let Ok(velocity) = player_query.get_single() {
            let speed = ((velocity.x * velocity.x + velocity.y * velocity.y) as f32).sqrt();
            let normal_speed = 200.0 * 1.414; // sqrt(200^2 + 200^2) for diagonal movement
            
            **text = format!("Velocity: ({:.1}, {:.1}) Speed: {:.1} - Press SPACE to speed hack!", 
                            velocity.x, velocity.y, speed);
            
            // Color code based on speed (red if hacking)
            text_color.0 = if speed > normal_speed + 10.0 {
                Color::srgb(1.0, 0.0, 0.0) // Red for speed hacking
            } else if speed > 10.0 {
                Color::srgb(0.0, 1.0, 0.0) // Green for normal movement
            } else {
                Color::srgb(0.8, 0.8, 0.8) // Gray for stationary
            };
        }
    }
}