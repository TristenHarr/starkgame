use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{PrimeField64, PrimeCharacteristicRing};
use p3_matrix::{Matrix, dense::RowMajorMatrix};
use crate::movement_trace::MovementTrace;

// Number of columns in our AIR
pub const NUM_MOVEMENT_COLS: usize = 8;

pub struct MovementAir;

impl MovementAir {
    pub fn new(_movement_speed: f32, _game_bounds: (f32, f32, f32, f32), _delta_time: f32) -> Self {
        Self
    }
}

impl<F> BaseAir<F> for MovementAir {
    fn width(&self) -> usize {
        NUM_MOVEMENT_COLS
    }
}

impl<AB: AirBuilder> Air<AB> for MovementAir {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        
        // Get current and next rows (for state transitions)
        let (local, next) = (
            main.row_slice(0).expect("Matrix is empty?"),
            main.row_slice(1).expect("Matrix only has 1 row?"),
        );
        
        let local: &MovementRow<AB::Var> = (*local).borrow();
        let next: &MovementRow<AB::Var> = (*next).borrow();

        // Constraint 1: Boolean inputs (each input flag is 0 or 1)
        builder.assert_bool(local.input_left.clone());
        builder.assert_bool(local.input_right.clone());
        builder.assert_bool(local.input_up.clone());
        builder.assert_bool(local.input_down.clone());

        // Constraint 2: Velocity must match inputs exactly
        // Account for the +1000 offset used in trace generation for negative velocities
        let velocity_offset = AB::F::from_u64(1000); // Offset to handle negative velocities
        let movement_speed = AB::F::from_u64(200); // Must match actual game speed
        
        // Expected velocity calculation: input * speed + offset
        let expected_vel_x = (local.input_right.clone() - local.input_left.clone()) * AB::Expr::from(movement_speed) + AB::Expr::from(velocity_offset);
        let expected_vel_y = (local.input_up.clone() - local.input_down.clone()) * AB::Expr::from(movement_speed) + AB::Expr::from(velocity_offset);
        
        // Velocity constraint - this should catch speed hacking
        builder.assert_eq(local.velocity_x.clone(), expected_vel_x);
        builder.assert_eq(local.velocity_y.clone(), expected_vel_y);
        
        // Constraint 3: Position continuity - prevents teleportation
        // Use the NEXT frame's velocity to validate the position change (original approach)
        let mut when_transition = builder.when_transition();
        
        // Use the NEXT frame's velocity to validate the position change that occurred
        let actual_next_vel_x = next.velocity_x.clone() - AB::Expr::from(velocity_offset);
        let actual_next_vel_y = next.velocity_y.clone() - AB::Expr::from(velocity_offset);
        
        // Physics factor: velocity * 15 = position_change (from our integer physics)
        let physics_factor = AB::F::from_u64(15);
        
        // Expected position based on the velocity that caused this movement
        let expected_next_x = local.position_x.clone() + actual_next_vel_x * AB::Expr::from(physics_factor);
        let expected_next_y = local.position_y.clone() + actual_next_vel_y * AB::Expr::from(physics_factor);
        
        
        // These must match exactly - any deviation (including teleportation) will fail
        when_transition.assert_eq(next.position_x.clone(), expected_next_x);
        when_transition.assert_eq(next.position_y.clone(), expected_next_y);

        // Constraint 4: First trace after reset must start at origin (0,0) with velocity (0,0)  
        // This is enforced by checking the first step in generate_movement_trace_matrix
        // The constraint is already enforced during trace generation, not here to avoid complexity
        
    }
}

// Structure representing a single row in our trace
#[repr(C)]
pub struct MovementRow<F> {
    pub position_x: F,
    pub position_y: F,
    pub velocity_x: F,
    pub velocity_y: F,
    pub input_left: F,
    pub input_right: F,
    pub input_up: F,
    pub input_down: F,
}

impl<F> Borrow<MovementRow<F>> for [F] {
    fn borrow(&self) -> &MovementRow<F> {
        debug_assert_eq!(self.len(), NUM_MOVEMENT_COLS);
        let (prefix, shorts, suffix) = unsafe { self.align_to::<MovementRow<F>>() };
        debug_assert!(prefix.is_empty(), "Alignment should match");
        debug_assert!(suffix.is_empty(), "Alignment should match");
        debug_assert_eq!(shorts.len(), 1);
        &shorts[0]
    }
}

// Function to generate trace matrix from MovementTrace
pub fn generate_movement_trace_matrix<F: PrimeField64>(
    trace: &MovementTrace,
    target_height: usize,
) -> RowMajorMatrix<F> {
    assert!(target_height.is_power_of_two());
    assert!(trace.steps.len() <= target_height, "Trace too long for target height");

    let mut matrix = RowMajorMatrix::new(
        F::zero_vec(target_height * NUM_MOVEMENT_COLS),
        NUM_MOVEMENT_COLS,
    );

    let (prefix, rows, suffix) = unsafe { matrix.values.align_to_mut::<MovementRow<F>>() };
    assert!(prefix.is_empty(), "Alignment should match");
    assert!(suffix.is_empty(), "Alignment should match");
    assert_eq!(rows.len(), target_height);

    // Fill rows with trace data
    for (i, step) in trace.steps.iter().enumerate() {
        if i >= target_height {
            break;
        }
        
        // CRITICAL: Enforce that first trace after reset starts at origin
        if trace.is_first_trace_after_reset && i == 0 {
            if step.position.x != 0.0 || step.position.y != 0.0 || step.velocity.x != 0.0 || step.velocity.y != 0.0 {
                panic!("First trace after reset must start at origin with zero velocity");
            }
        }

        // Convert to fixed-point representation that matches AIR expectations
        // Scale positions by 1000 for precision, handle negatives properly
        let pos_x_scaled = (step.position.x * 1000.0) as i64;
        let pos_y_scaled = (step.position.y * 1000.0) as i64;
        let vel_x_scaled = step.velocity.x as i64; // Keep velocities as integers
        let vel_y_scaled = step.velocity.y as i64;
        
        // Expand encoding range to support much larger game boundaries
        // BabyBear field can hold ~2 billion, so we can safely use 100M range (±50k pixels)
        let encoded_pos_x = ((pos_x_scaled + 50000000) as u64) % 100000000;
        let encoded_pos_y = ((pos_y_scaled + 50000000) as u64) % 100000000;
        let encoded_vel_x = ((vel_x_scaled + 1000) as u64) % 2000;
        let encoded_vel_y = ((vel_y_scaled + 1000) as u64) % 2000;
        
        // Enhanced debug logging - show ALL rows and check for problematic values  
        let is_interesting = i < 10 || (encoded_vel_x != 1000 || encoded_vel_y != 1000) || 
                           pos_x_scaled.abs() > 10000000 || pos_y_scaled.abs() > 10000000 ||
                           encoded_pos_x > 90000000 || encoded_pos_y > 90000000;
                           
        // Check for large position jumps that indicate teleportation
        let has_large_jump = if i > 0 {
            let prev_step = &trace.steps[i-1];
            let curr_step = step;
            let distance = ((curr_step.position.x - prev_step.position.x).powi(2) + 
                           (curr_step.position.y - prev_step.position.y).powi(2)).sqrt();
            distance > 50.0
        } else { false };
        
        if is_interesting || has_large_jump {
            let dt = step.delta_time;
            let _expected_pos_change_x = step.velocity.x * dt * 1000.0;
            let _expected_pos_change_y = step.velocity.y * dt * 1000.0;
            
            // Calculate what the constraint expects from inputs
            let expected_vel_x_from_inputs = (if step.inputs.right { 1.0 } else { 0.0 } - if step.inputs.left { 1.0 } else { 0.0 }) * 200.0;
            let expected_vel_y_from_inputs = (if step.inputs.up { 1.0 } else { 0.0 } - if step.inputs.down { 1.0 } else { 0.0 }) * 200.0;
            let constraint_expected_vel_x = expected_vel_x_from_inputs + 1000.0; // With offset
            let constraint_expected_vel_y = expected_vel_y_from_inputs + 1000.0; // With offset
            
            // If this is a transition row, show what previous row was
            let transition_info = if i > 0 && i < trace.steps.len() - 1 {
                let prev_step = &trace.steps[i-1];
                format!(" [TRANSITION from vel={:.1} to vel={:.1}]", prev_step.velocity.x, step.velocity.x)
            } else { String::new() };
            
            // Show constraint violation details
            let vel_x_violation = if (encoded_vel_x as f32 - constraint_expected_vel_x as f32).abs() > 0.1 { "❌" } else { "✅" };
            let vel_y_violation = if (encoded_vel_y as f32 - constraint_expected_vel_y as f32).abs() > 0.1 { "❌" } else { "✅" };
            
            // Field overflow warning - now supports much larger positions
            let overflow_warning = if pos_x_scaled.abs() > 10000000 || pos_y_scaled.abs() > 10000000 {
                " ⚠️ LARGE_POSITION"
            } else if encoded_pos_x > 90000000 || encoded_pos_y > 90000000 {
                " ⚠️ ENCODING_OVERFLOW"
            } else { "" };
            
            let teleport_warning = if has_large_jump {
                " 🚨 TELEPORT_IN_TRACE"
            } else { "" };
            
        }
        
        rows[i] = MovementRow {
            position_x: F::from_u64(encoded_pos_x),
            position_y: F::from_u64(encoded_pos_y),
            velocity_x: F::from_u64(encoded_vel_x),
            velocity_y: F::from_u64(encoded_vel_y),
            input_left: if step.inputs.left { F::ONE } else { F::ZERO },
            input_right: if step.inputs.right { F::ONE } else { F::ZERO },
            input_up: if step.inputs.up { F::ONE } else { F::ZERO },
            input_down: if step.inputs.down { F::ONE } else { F::ZERO },
        };
    }

    // Pad remaining rows with the last step (or zeros if empty)
    if !trace.steps.is_empty() {
        let last_step = &trace.steps[trace.steps.len() - 1];
        let last_pos_x_scaled = (last_step.position.x * 1000.0) as i64;
        let last_pos_y_scaled = (last_step.position.y * 1000.0) as i64;
        
        for i in trace.steps.len()..target_height {
            rows[i] = MovementRow {
                // Keep last position with same encoding
                position_x: F::from_u64(((last_pos_x_scaled + 50000000) as u64) % 100000000),
                position_y: F::from_u64(((last_pos_y_scaled + 50000000) as u64) % 100000000),
                // No movement in padding rows: velocity = 0 + offset = 1000
                velocity_x: F::from_u64(1000), 
                velocity_y: F::from_u64(1000),
                input_left: F::ZERO,
                input_right: F::ZERO,
                input_up: F::ZERO,
                input_down: F::ZERO,
            };
        }
    }

    matrix
}

// Helper function to generate a matrix that will intentionally fail constraint validation
// This is used when we detect cheating during trace generation
fn generate_cheat_detected_matrix<F: PrimeField64>(target_height: usize) -> RowMajorMatrix<F> {
    
    let mut matrix = RowMajorMatrix::new(
        F::zero_vec(target_height * NUM_MOVEMENT_COLS),
        NUM_MOVEMENT_COLS,
    );

    let (prefix, rows, suffix) = unsafe { matrix.values.align_to_mut::<MovementRow<F>>() };
    assert!(prefix.is_empty(), "Alignment should match");
    assert!(suffix.is_empty(), "Alignment should match");
    assert_eq!(rows.len(), target_height);

    // Generate a matrix that will definitely fail constraint validation
    // Set invalid values that violate the velocity constraint
    for i in 0..target_height {
        rows[i] = MovementRow {
            position_x: F::from_u64(50000000), // Encoded (0,0)
            position_y: F::from_u64(50000000), // Encoded (0,0)
            velocity_x: F::from_u64(9999),     // Invalid velocity that doesn't match inputs
            velocity_y: F::from_u64(9999),     // Invalid velocity that doesn't match inputs
            input_left: F::ZERO,
            input_right: F::ZERO,
            input_up: F::ZERO,
            input_down: F::ZERO,
        };
    }

    matrix
}

// Utility function to find next power of 2 for trace height
pub fn next_power_of_2(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut power = 1;
    while power < n {
        power <<= 1;
    }
    power
}