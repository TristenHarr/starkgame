use bevy::prelude::*;
use std::collections::VecDeque;
use crate::{Position, Velocity, Player, LastInputState};

#[derive(Clone, Debug)]
pub struct MovementStep {
    pub position: Vec2,
    pub velocity: Vec2,
    pub inputs: InputFlags,
    pub timestamp: f64,
    pub delta_time: f32, // Actual frame delta time
}

#[derive(Clone, Debug, Default)]
pub struct InputFlags {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

impl InputFlags {
    pub fn from_keyboard(keyboard: &Res<ButtonInput<KeyCode>>) -> Self {
        Self {
            left: keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA),
            right: keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD),
            up: keyboard.pressed(KeyCode::ArrowUp) || keyboard.pressed(KeyCode::KeyW),
            down: keyboard.pressed(KeyCode::ArrowDown) || keyboard.pressed(KeyCode::KeyS),
        }
    }

    pub fn to_velocity(&self, speed: f32) -> Vec2 {
        let mut velocity = Vec2::ZERO;
        
        if self.left { velocity.x -= speed; }
        if self.right { velocity.x += speed; }
        if self.up { velocity.y += speed; }
        if self.down { velocity.y -= speed; }
        
        velocity
    }
}

#[derive(Clone, Debug)]
pub struct MovementTrace {
    pub steps: Vec<MovementStep>,
    pub start_time: f64,
    pub duration: f64,
}

impl MovementTrace {
    pub fn new(start_time: f64) -> Self {
        Self {
            steps: Vec::new(),
            start_time,
            duration: 0.0,
        }
    }

    pub fn add_step(&mut self, step: MovementStep) {
        self.duration = step.timestamp - self.start_time;
        self.steps.push(step);
    }

    pub fn is_complete(&self, target_duration: f64) -> bool {
        self.duration >= target_duration
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }
}

#[derive(Component, Default)]
pub struct MovementTraceCollector {
    pub current_trace: Option<MovementTrace>,
    pub completed_traces: VecDeque<MovementTrace>,
    pub trace_duration: f64,
    pub max_completed_traces: usize,
}

impl MovementTraceCollector {
    pub fn new(trace_duration: f64, max_completed_traces: usize) -> Self {
        Self {
            current_trace: None,
            completed_traces: VecDeque::new(),
            trace_duration,
            max_completed_traces,
        }
    }

    pub fn start_new_trace(&mut self, timestamp: f64) {
        self.current_trace = Some(MovementTrace::new(timestamp));
    }

    pub fn add_movement(&mut self, position: Vec2, velocity: Vec2, inputs: InputFlags, timestamp: f64) {
        if self.current_trace.is_none() {
            self.start_new_trace(timestamp);
        }

        if let Some(ref mut trace) = self.current_trace {
            let step = MovementStep {
                position,
                velocity,
                inputs,
                timestamp,
                delta_time: 0.016, // Fixed for now
            };
            
            trace.add_step(step);

            if trace.is_complete(self.trace_duration) {
                self.complete_current_trace();
            }
        }
    }

    pub fn complete_current_trace(&mut self) {
        if let Some(trace) = self.current_trace.take() {
            self.completed_traces.push_back(trace);
            
            while self.completed_traces.len() > self.max_completed_traces {
                self.completed_traces.pop_front();
            }
        }
    }

    pub fn get_next_trace_for_proving(&mut self) -> Option<MovementTrace> {
        self.completed_traces.pop_front()
    }

    pub fn has_traces_to_prove(&self) -> bool {
        !self.completed_traces.is_empty()
    }
}

pub fn movement_trace_collection_system(
    time: Res<Time>,
    mut query: Query<(&Position, &Velocity, &LastInputState, &mut MovementTraceCollector), With<Player>>,
) {
    let current_time = time.elapsed_secs_f64();

    for (position, velocity, input_state, mut collector) in &mut query {
        // Use the stored input state that was captured when velocity was set
        // This ensures perfect synchronization between inputs and velocity
        let synchronized_inputs = InputFlags {
            left: input_state.left,
            right: input_state.right,
            up: input_state.up,
            down: input_state.down,
        };

        // Log trace collection details
        if velocity.x != 0 || velocity.y != 0 {
            info!("📊 TRACE_COLLECTION: pos=({:.1},{:.1}) vel=({},{}) inputs=({},{},{},{}) → adding to trace", 
                  position.x as f32, position.y as f32, velocity.x, velocity.y,
                  if synchronized_inputs.left { 1 } else { 0 },
                  if synchronized_inputs.right { 1 } else { 0 },
                  if synchronized_inputs.up { 1 } else { 0 },
                  if synchronized_inputs.down { 1 } else { 0 });
        }

        collector.add_movement(
            Vec2::new(position.x as f32, position.y as f32),
            Vec2::new(velocity.x as f32, velocity.y as f32),
            synchronized_inputs,
            current_time,
        );
    }
}

#[derive(Resource)]
pub struct TraceSettings {
    pub trace_duration: f64,
    pub max_completed_traces: usize,
}

impl Default for TraceSettings {
    fn default() -> Self {
        Self {
            trace_duration: 1.0, // 1 second traces
            max_completed_traces: 5,
        }
    }
}