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


#[derive(Clone, Debug)]
pub struct MovementTrace {
    pub steps: Vec<MovementStep>,
    pub start_time: f64,
    pub duration: f64,
    pub is_first_trace_after_reset: bool, // Flag to mark first trace after game reset
}

impl MovementTrace {
    pub fn new(start_time: f64) -> Self {
        Self {
            steps: Vec::new(),
            start_time,
            duration: 0.0,
            is_first_trace_after_reset: false,
        }
    }
    
    pub fn new_first_after_reset(start_time: f64) -> Self {
        Self {
            steps: Vec::new(),
            start_time,
            duration: 0.0,
            is_first_trace_after_reset: true,
        }
    }

    pub fn add_step(&mut self, step: MovementStep) {
        self.duration = step.timestamp - self.start_time;
        self.steps.push(step);
    }

    pub fn is_complete(&self, target_duration: f64) -> bool {
        self.duration >= target_duration
    }

}

#[derive(Component, Default)]
pub struct MovementTraceCollector {
    pub current_trace: Option<MovementTrace>,
    pub completed_traces: VecDeque<MovementTrace>,
    pub trace_duration: f64,
    pub max_completed_traces: usize,
    pub next_trace_is_first_after_reset: bool, // Flag to mark next trace as first after reset
}

impl MovementTraceCollector {
    pub fn new(trace_duration: f64, max_completed_traces: usize) -> Self {
        Self {
            current_trace: None,
            completed_traces: VecDeque::new(),
            trace_duration,
            max_completed_traces,
            next_trace_is_first_after_reset: true, // First trace after startup is also first after reset
        }
    }

    pub fn start_new_trace(&mut self, timestamp: f64) {
        if self.next_trace_is_first_after_reset {
            self.current_trace = Some(MovementTrace::new_first_after_reset(timestamp));
            self.next_trace_is_first_after_reset = false;
        } else {
            self.current_trace = Some(MovementTrace::new(timestamp));
        }
    }
    
    pub fn mark_next_trace_as_first_after_reset(&mut self) {
        self.next_trace_is_first_after_reset = true;
    }

    pub fn add_movement(&mut self, position: Vec2, velocity: Vec2, inputs: InputFlags, timestamp: f64) {
        // CRITICAL FIX: Always ensure we have a trace active, even if the previous one just completed
        if self.current_trace.is_none() {
            self.start_new_trace(timestamp);
        }

        if let Some(ref mut trace) = self.current_trace {
            let step = MovementStep {
                position,
                velocity,
                inputs: inputs.clone(),
                timestamp,
                delta_time: 0.016, // Fixed for now
            };
            
            trace.add_step(step);

            // CRITICAL FIX: If trace is complete, start a new one IMMEDIATELY with this same step
            // This prevents any position changes from falling between trace boundaries
            if trace.is_complete(self.trace_duration) {
                self.complete_current_trace();
                
                // Immediately start a new trace and add this step to it as well
                // This ensures continuity - no position change can escape being traced
                self.start_new_trace(timestamp);
                if let Some(ref mut new_trace) = self.current_trace {
                    let continuation_step = MovementStep {
                        position,
                        velocity,
                        inputs,
                        timestamp,
                        delta_time: 0.016,
                    };
                    new_trace.add_step(continuation_step);
                }
            }
        }
    }

    pub fn complete_current_trace(&mut self) {
        if let Some(trace) = self.current_trace.take() {
            // Check if this trace contains any large position jumps
            let mut has_teleport = false;
            for i in 1..trace.steps.len() {
                let prev = &trace.steps[i-1];
                let curr = &trace.steps[i];
                let distance = prev.position.distance(curr.position);
                if distance > 50.0 {  // Definitely a teleport
                    has_teleport = true;
                }
            }
            
            
            self.completed_traces.push_back(trace);
            
            while self.completed_traces.len() > self.max_completed_traces {
                self.completed_traces.pop_front();
            }
        }
    }

    pub fn get_next_trace_for_proving(&mut self) -> Option<MovementTrace> {
        self.completed_traces.pop_front()
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


        // Add detailed position tracking
        let pos_vec = Vec2::new(position.x as f32, position.y as f32);
        let vel_vec = Vec2::new(velocity.x as f32, velocity.y as f32);
        
        collector.add_movement(pos_vec, vel_vec, synchronized_inputs, current_time);
        
    }
}

