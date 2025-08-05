use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::winit::WinitSettings;
use crate::{Player, ProofGenerator, Velocity};

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct ProofStatsText;

#[derive(Component)]
struct VelocityText;

pub struct FpsDisplayPlugin;

impl Plugin for FpsDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(WinitSettings {
                focused_mode: bevy::winit::UpdateMode::Continuous,
                unfocused_mode: bevy::winit::UpdateMode::Continuous,
            })
            .add_systems(Startup, setup_fps_display)
            .add_systems(Update, (
                update_fps_display, 
                update_proof_stats_display,
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
            
            let avg_verification_time = proof_gen.stats.avg_verification_time();
            **text = format!(
                "Proofs: Active: {}, Generated: {}, Avg Gen: {:.1}ms, Avg Verify: {:.1}ms", 
                active_count, generated_count, avg_time / 1_000_000.0, avg_verification_time / 1_000_000.0
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