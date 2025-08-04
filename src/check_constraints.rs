use p3_field::PrimeCharacteristicRing;
use p3_matrix::Matrix;
use core::borrow::Borrow;
use crate::movement_air::{MovementAir, MovementRow};

type Val = p3_baby_bear::BabyBear;

// Self-contained constraint checking - replicates our MovementAir constraints
// This doesn't depend on any Plonky3 modifications
pub fn check_movement_constraints(air: &MovementAir, trace_matrix: &p3_matrix::dense::RowMajorMatrix<Val>) -> Result<(), String> {
    let height = trace_matrix.height();
    println!("🔍 CONSTRAINT_DEBUG: Checking {} rows for violations", height);
    
    let mut violations_found = 0;
    
    for row_index in 0..height {
        let row_index_next = (row_index + 1) % height;
        
        // Get current and next rows
        let local_row = trace_matrix.row_slice(row_index).ok_or("Failed to get local row")?;
        let next_row = trace_matrix.row_slice(row_index_next).ok_or("Failed to get next row")?;
        
        let local: &MovementRow<Val> = (&*local_row).borrow();
        let next: &MovementRow<Val> = (&*next_row).borrow();
        
        // Constraint 1: Boolean inputs (each input flag is 0 or 1)
        if local.input_left != Val::ZERO && local.input_left != Val::ONE {
            return Err(format!("Row {}: input_left {} is not boolean", row_index, local.input_left));
        }
        if local.input_right != Val::ZERO && local.input_right != Val::ONE {
            return Err(format!("Row {}: input_right {} is not boolean", row_index, local.input_right));
        }
        if local.input_up != Val::ZERO && local.input_up != Val::ONE {
            return Err(format!("Row {}: input_up {} is not boolean", row_index, local.input_up));
        }
        if local.input_down != Val::ZERO && local.input_down != Val::ONE {
            return Err(format!("Row {}: input_down {} is not boolean", row_index, local.input_down));
        }

        // Constraint 2: Velocity must match inputs exactly
        let velocity_offset = Val::from_u64(1000);
        let movement_speed = Val::from_u64(200);
        
        let expected_vel_x = (local.input_right - local.input_left) * movement_speed + velocity_offset;
        let expected_vel_y = (local.input_up - local.input_down) * movement_speed + velocity_offset;
        
        // Log every row that has movement or input
        if local.velocity_x != velocity_offset || local.velocity_y != velocity_offset || 
           local.input_left != Val::ZERO || local.input_right != Val::ZERO || 
           local.input_up != Val::ZERO || local.input_down != Val::ZERO {
            println!("🔍 CONSTRAINT Row {}: pos=({},{}) vel=({},{}) inputs=({},{},{},{}) expected_vel=({},{})", 
                     row_index, local.position_x, local.position_y, 
                     local.velocity_x, local.velocity_y,
                     local.input_left, local.input_right, local.input_up, local.input_down,
                     expected_vel_x, expected_vel_y);
        }
        
        if local.velocity_x != expected_vel_x {
            violations_found += 1;
            println!("❌ CONSTRAINT VIOLATION Row {}: velocity_x {} != expected {} (speed hacking detected)", 
                     row_index, local.velocity_x, expected_vel_x);
            panic!("🚨 CHEATING DETECTED! 🚨 Row {}: velocity_x {} != expected {} (speed hacking detected) - GAME TERMINATED", 
                   row_index, local.velocity_x, expected_vel_x);
        }
        
        if local.velocity_y != expected_vel_y {
            violations_found += 1;
            println!("❌ CONSTRAINT VIOLATION Row {}: velocity_y {} != expected {} (speed hacking detected)", 
                     row_index, local.velocity_y, expected_vel_y);
            panic!("🚨 CHEATING DETECTED! 🚨 Row {}: velocity_y {} != expected {} (speed hacking detected) - GAME TERMINATED", 
                   row_index, local.velocity_y, expected_vel_y);
        }
        
        // Constraint 3: Position continuity - prevents teleportation
        if row_index != height - 1 { // Not the last row (no wraparound)
            let actual_next_vel_x = next.velocity_x - velocity_offset;
            let actual_next_vel_y = next.velocity_y - velocity_offset;
            let physics_factor = Val::from_u64(15);
            
            let expected_next_x = local.position_x + actual_next_vel_x * physics_factor;
            let expected_next_y = local.position_y + actual_next_vel_y * physics_factor;
            
            // Log position transitions
            if actual_next_vel_x != Val::ZERO || actual_next_vel_y != Val::ZERO {
                println!("🎯 CONSTRAINT Transition {}->{}: pos ({},{}) + vel({},{}) * 15 = expected ({},{}) vs actual ({},{})", 
                         row_index, row_index + 1,
                         local.position_x, local.position_y,
                         actual_next_vel_x, actual_next_vel_y,
                         expected_next_x, expected_next_y,
                         next.position_x, next.position_y);
            }
            
            if next.position_x != expected_next_x {
                violations_found += 1;
                println!("❌ CONSTRAINT VIOLATION Row {}: position_x {} != expected {} (teleportation detected)", 
                         row_index, next.position_x, expected_next_x);
                panic!("🚨 CHEATING DETECTED! 🚨 Row {}: position_x {} != expected {} (teleportation detected) - GAME TERMINATED", 
                       row_index, next.position_x, expected_next_x);
            }
            
            if next.position_y != expected_next_y {
                violations_found += 1;
                println!("❌ CONSTRAINT VIOLATION Row {}: position_y {} != expected {} (teleportation detected)", 
                         row_index, next.position_y, expected_next_y);
                panic!("🚨 CHEATING DETECTED! 🚨 Row {}: position_y {} != expected {} (teleportation detected) - GAME TERMINATED", 
                       row_index, next.position_y, expected_next_y);
            }
        }
    }
    
    println!("✅ CONSTRAINT_DEBUG: All {} rows passed, {} violations found", height, violations_found);
    Ok(())
}