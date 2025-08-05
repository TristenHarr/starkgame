use p3_baby_bear::{BabyBear, Poseidon2BabyBear};
use p3_challenger::DuplexChallenger;
use p3_commit::ExtensionMmcs;
use p3_dft::Radix2DitParallel;
use p3_field::extension::BinomialExtensionField;
use p3_fri::{TwoAdicFriPcs, create_test_fri_params};
use p3_merkle_tree::MerkleTreeMmcs;
use p3_symmetric::{PaddingFreeSponge, TruncatedPermutation};
use p3_uni_stark::{StarkConfig, prove, verify};
use p3_matrix::Matrix;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use std::time::Instant;
use futures_lite::future;

use crate::movement_air::{MovementAir, generate_movement_trace_matrix, next_power_of_2};
use crate::movement_trace::{MovementTrace, MovementTraceCollector};
use crate::Player;

// Type aliases for our STARK configuration
type Val = BabyBear;
type Perm = Poseidon2BabyBear<16>;
type MyHash = PaddingFreeSponge<Perm, 16, 8, 8>;
type MyCompress = TruncatedPermutation<Perm, 2, 8, 16>;
type ValMmcs = MerkleTreeMmcs<<Val as p3_field::Field>::Packing, <Val as p3_field::Field>::Packing, MyHash, MyCompress, 8>;
type Challenge = BinomialExtensionField<Val, 4>;
type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
type Challenger = DuplexChallenger<Val, Perm, 16, 8>;
type Dft = Radix2DitParallel<Val>;
type Pcs = TwoAdicFriPcs<Val, Dft, ValMmcs, ChallengeMmcs>;
type MyConfig = StarkConfig<Pcs, Challenge, Challenger>;

#[derive(Resource)]
pub struct ProofSystemSettings {
    pub movement_speed: f32,
    pub game_bounds: (f32, f32, f32, f32),
    pub delta_time: f32,
}

impl Default for ProofSystemSettings {
    fn default() -> Self {
        Self {
            movement_speed: 200.0, // pixels per second
            game_bounds: (-400.0, 400.0, -300.0, 300.0), // Window bounds
            delta_time: 1.0 / 60.0, // 60 FPS
        }
    }
}

fn create_stark_config() -> (MyConfig, MovementAir) {
    let mut rng = SmallRng::seed_from_u64(42); // Fixed seed for reproducibility
    let perm = Perm::new_from_rng_128(&mut rng);
    let hash = MyHash::new(perm.clone());
    let compress = MyCompress::new(perm.clone());
    let val_mmcs = ValMmcs::new(hash, compress);
    let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());
    let dft = Dft::default();
    
    let fri_params = create_test_fri_params(challenge_mmcs, 2);
    let pcs = Pcs::new(dft, val_mmcs, fri_params);
    let challenger = Challenger::new(perm);
    
    let config = MyConfig::new(pcs, challenger);
    
    // Game configuration - using defaults for now
    let settings = ProofSystemSettings::default();
    let air = MovementAir::new(settings.movement_speed, settings.game_bounds, settings.delta_time);
    
    (config, air)
}

#[derive(Component)]
pub struct ProofGenerator {
    pub active_tasks: Vec<Task<ProofResult>>, 
    pub completed_count: usize,
    pub stats: ProofStats,
}

#[derive(Debug)]
pub struct ProofResult {
    pub result: Result<(Vec<u8>, usize), String>, // (proof_bytes, size) or error
    pub generation_time_ms: f64,
    pub verification_time_ms: f64,
}



#[derive(Debug, Default)]
pub struct ProofStats {
    pub total_proofs_generated: usize,
    pub total_generation_time_ms: f64,
    pub total_verification_time_ms: f64,
    pub successful_verifications: usize,
    pub failed_verifications: usize,
}

impl ProofStats {
    pub fn avg_generation_time(&self) -> f64 {
        if self.total_proofs_generated > 0 {
            self.total_generation_time_ms / self.total_proofs_generated as f64
        } else {
            0.0
        }
    }

    pub fn avg_verification_time(&self) -> f64 {
        let total_verifications = self.successful_verifications + self.failed_verifications;
        if total_verifications > 0 {
            self.total_verification_time_ms / total_verifications as f64
        } else {
            0.0
        }
    }
}

impl Default for ProofGenerator {
    fn default() -> Self {
        Self {
            active_tasks: Vec::new(),
            completed_count: 0,
            stats: ProofStats::default(),
        }
    }
}

pub fn proof_generation_system(
    time: Res<Time>,
    mut query: Query<(&mut MovementTraceCollector, &mut ProofGenerator), With<Player>>,
    mut commands: Commands,
) {
    let _current_time = time.elapsed_secs_f64();

    for (mut collector, mut proof_gen) in &mut query {
        // Check for completed traces to prove and start async tasks
        while let Some(trace) = collector.get_next_trace_for_proving() {
            if trace.steps.len() > 1 {
                // Check if this trace contains teleportation
                let mut max_jump: f32 = 0.0;
                for i in 1..trace.steps.len() {
                    let distance = trace.steps[i-1].position.distance(trace.steps[i].position);
                    max_jump = max_jump.max(distance);
                }
                
                
                // Start async proof generation task
                let task_pool = AsyncComputeTaskPool::get();
                let trace_clone = trace.clone();
                
                #[allow(unused_must_use)]
                let task = task_pool.spawn(async move {
                    let generation_start = Instant::now();
                    
                    // Generate proof on background thread
                    let (result, verification_time) = generate_proof_async(&trace_clone).await;
                    let generation_time = generation_start.elapsed().as_millis() as f64;
                    
                    ProofResult {
                        result,
                        generation_time_ms: generation_time,
                        verification_time_ms: verification_time,
                    }
                });
                
                proof_gen.active_tasks.push(task);
            }
        }

        // Check for completed async tasks (non-blocking)
        let mut i = 0;
        while i < proof_gen.active_tasks.len() {
            if let Some(result) = future::block_on(future::poll_once(&mut proof_gen.active_tasks[i])) {
                // Task completed, remove it and process result
                let _ = proof_gen.active_tasks.remove(i);
                
                match result.result {
                    Ok((_proof_bytes, proof_size)) => {
                        
                        // Update statistics
                        proof_gen.stats.total_proofs_generated += 1;
                        proof_gen.stats.total_generation_time_ms += result.generation_time_ms;
                        proof_gen.stats.total_verification_time_ms += result.verification_time_ms;
                        proof_gen.stats.successful_verifications += 1;
                        
                        proof_gen.completed_count += 1;
                    }
                    Err(e) => {
                        if e.starts_with("CHEAT_DETECTED:") {
                            error!("🚨 CHEAT DETECTED: {}", e);
                        } else {
                            error!("❌ Async proof generation failed: {}", e);
                        }
                        // Increment failed verifications - cheat_detection_system will handle state transition
                        proof_gen.stats.failed_verifications += 1;
                    }
                }
            } else {
                i += 1; // Task still running, check next one
            }
        }
    }
}

async fn generate_proof_async(trace: &MovementTrace) -> (Result<(Vec<u8>, usize), String>, f64) {
    // Create STARK config inside the async function (each task gets its own)
    let (config, air) = create_stark_config();
    
    // Find appropriate trace height (next power of 2)
    let target_height = next_power_of_2(trace.steps.len().max(8));
    
    // Generate trace matrix and proof - catch panics from both operations
    let proof_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let trace_matrix = generate_movement_trace_matrix::<Val>(trace, target_height);
        prove(&config, &air, trace_matrix, &vec![])
    }));
    
    let proof = match proof_result {
        Ok(proof) => proof,
        Err(_panic_info) => {
            return (Err("CHEAT_DETECTED: Constraint violation during proof generation".to_string()), 0.0);
        }
    };
    
    // Serialize proof to get size
    let proof_bytes = match bincode::serialize(&proof) {
        Ok(bytes) => bytes,
        Err(e) => return (Err(format!("Proof serialization failed: {:?}", e)), 0.0),
    };
    
    let proof_size = proof_bytes.len();
    
    // VERIFY THE PROOF - this is critical for anti-cheat!
    let verification_start = Instant::now();
    let verification_result = match bincode::deserialize::<_>(&proof_bytes) {
        Ok(deserialized_proof) => {
            match verify(&config, &air, &deserialized_proof, &vec![]) {
                Ok(_) => Ok((proof_bytes, proof_size)),
                Err(e) => Err(format!("CHEAT_DETECTED: Invalid proof: {:?}", e))
            }
        }
        Err(e) => Err(format!("CHEAT_DETECTED: Corrupted proof: {:?}", e))
    };
    let verification_time = verification_start.elapsed().as_millis() as f64;
    
    (verification_result, verification_time)
}




pub fn stats_logging_system(
    _time: Res<Time>,
    _query: Query<&ProofGenerator, With<Player>>,
) {
    // Stats logging disabled for cleaner output
}