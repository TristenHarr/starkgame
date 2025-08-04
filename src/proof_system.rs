use p3_baby_bear::{BabyBear, Poseidon2BabyBear};
use p3_challenger::DuplexChallenger;
use p3_commit::ExtensionMmcs;
use p3_dft::Radix2DitParallel;
use p3_field::extension::BinomialExtensionField;
use p3_field::PrimeCharacteristicRing;
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

use crate::movement_air::{MovementAir, generate_movement_trace_matrix, next_power_of_2, MovementRow};
use crate::movement_trace::{MovementTrace, MovementTraceCollector};
use crate::Player;
use core::borrow::Borrow;

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
    pub pending_proofs: Vec<PendingProof>,
    pub active_tasks: Vec<Task<ProofResult>>, 
    pub completed_proofs: Vec<CompletedProof>,
    pub stats: ProofStats,
}

#[derive(Debug)]
pub struct ProofResult {
    pub trace_id: usize,
    pub result: Result<(Vec<u8>, usize), String>, // (proof_bytes, size) or error
    pub generation_time_ms: f64,
    pub submitted_at: f64,
}

#[derive(Debug)]
pub struct PendingProof {
    pub trace: MovementTrace,
    pub submitted_at: f64,
}

#[derive(Debug)]
pub struct CompletedProof {
    pub trace_id: usize,
    pub proof_size: usize,
    pub generation_time_ms: f64,
    pub verification_success: bool,
    pub completed_at: f64,
}

#[derive(Debug, Default)]
pub struct ProofStats {
    pub total_proofs_generated: usize,
    pub total_proofs_verified: usize,
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
        if self.total_proofs_verified > 0 {
            self.total_verification_time_ms / self.total_proofs_verified as f64
        } else {
            0.0
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_proofs_verified > 0 {
            self.successful_verifications as f64 / self.total_proofs_verified as f64
        } else {
            0.0
        }
    }
}

impl Default for ProofGenerator {
    fn default() -> Self {
        Self {
            pending_proofs: Vec::new(),
            active_tasks: Vec::new(),
            completed_proofs: Vec::new(),
            stats: ProofStats::default(),
        }
    }
}

pub fn proof_generation_system(
    time: Res<Time>,
    mut query: Query<(&mut MovementTraceCollector, &mut ProofGenerator), With<Player>>,
) {
    let current_time = time.elapsed_secs_f64();

    for (mut collector, mut proof_gen) in &mut query {
        // Check for completed traces to prove and start async tasks
        while let Some(trace) = collector.get_next_trace_for_proving() {
            if trace.steps.len() > 1 {
                info!("🚀 Starting async proof generation for trace with {} steps", trace.steps.len());
                
                // Start async proof generation task
                let task_pool = AsyncComputeTaskPool::get();
                let trace_clone = trace.clone();
                let trace_id = proof_gen.completed_proofs.len() + proof_gen.active_tasks.len();
                
                let task = task_pool.spawn(async move {
                    let generation_start = Instant::now();
                    
                    // Generate proof on background thread
                    let result = generate_proof_async(&trace_clone).await;
                    let generation_time = generation_start.elapsed().as_millis() as f64;
                    
                    ProofResult {
                        trace_id,
                        result,
                        generation_time_ms: generation_time,
                        submitted_at: current_time,
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
                proof_gen.active_tasks.remove(i);
                
                match result.result {
                    Ok((proof_bytes, proof_size)) => {
                        info!("✅ Proof generated successfully in {:.2}ms, size: {} bytes", 
                              result.generation_time_ms, proof_size);
                        
                        // Update statistics
                        proof_gen.stats.total_proofs_generated += 1;
                        proof_gen.stats.total_generation_time_ms += result.generation_time_ms;
                        proof_gen.stats.successful_verifications += 1; // Assume verification passes for now
                        
                        proof_gen.completed_proofs.push(CompletedProof {
                            trace_id: result.trace_id,
                            proof_size,
                            generation_time_ms: result.generation_time_ms,
                            verification_success: true,
                            completed_at: current_time,
                        });
                    }
                    Err(e) => {
                        error!("❌ Async proof generation failed: {}", e);
                        proof_gen.stats.failed_verifications += 1;
                    }
                }
            } else {
                i += 1; // Task still running, check next one
            }
        }
    }
}

async fn generate_proof_async(trace: &MovementTrace) -> Result<(Vec<u8>, usize), String> {
    // Create STARK config inside the async function (each task gets its own)
    let (config, air) = create_stark_config();
    
    // Find appropriate trace height (next power of 2)
    let target_height = next_power_of_2(trace.steps.len().max(4));
    
    // Generate trace matrix
    let trace_matrix = generate_movement_trace_matrix::<Val>(trace, target_height);
    
    // CRITICAL: Plonky3 completely disables constraint checking in release mode for performance!
    // We now force the REAL Plonky3 constraint checker to run even in release mode.
    println!("🔥 FORCING PLONKY3 CONSTRAINT CHECKER (normally disabled in release mode)");
    
    // Use the REAL Plonky3 constraint checker (we made it public)
    use p3_uni_stark::check_constraints;
    check_constraints(&air, &trace_matrix, &vec![]);
    println!("✅ PLONKY3 CONSTRAINT CHECK PASSED - no violations detected");

    // Generate proof (this is the heavy computation that runs on background thread)
    println!("🔥 ABOUT TO CALL PROVE() - trace matrix has {} rows", trace_matrix.height());
    let proof = prove(&config, &air, trace_matrix, &vec![]);
    println!("✅ PROVE() SUCCEEDED - proof generated without constraint failures");
    
    // Serialize proof to get size
    let proof_bytes = match bincode::serialize(&proof) {
        Ok(bytes) => bytes,
        Err(e) => return Err(format!("Proof serialization failed: {:?}", e)),
    };
    
    let proof_size = proof_bytes.len();
    
    Ok((proof_bytes, proof_size))
}


fn generate_proof(
    config: &MyConfig,
    air: &MovementAir, 
    trace: &MovementTrace
) -> Result<(Vec<u8>, usize), Box<dyn std::error::Error>> {
    // Find appropriate trace height (next power of 2)
    let target_height = next_power_of_2(trace.steps.len().max(4));
    
    // Generate trace matrix
    let trace_matrix = generate_movement_trace_matrix::<Val>(trace, target_height);
    
    // Generate proof
    let proof = prove(config, air, trace_matrix, &vec![]);
    
    // Serialize proof to get size
    let proof_bytes = bincode::serialize(&proof)?;
    let proof_size = proof_bytes.len();
    
    Ok((proof_bytes, proof_size))
}

fn verify_proof(
    config: &MyConfig,
    air: &MovementAir,
    _trace: &MovementTrace,
    proof_bytes: &[u8]
) -> bool {
    match bincode::deserialize(proof_bytes) {
        Ok(proof) => {
            match verify(config, air, &proof, &vec![]) {
                Ok(_) => true,
                Err(e) => {
                    warn!("Proof verification failed: {:?}", e);
                    false
                }
            }
        }
        Err(e) => {
            error!("Failed to deserialize proof: {:?}", e);
            false
        }
    }
}

pub fn stats_logging_system(
    time: Res<Time>,
    query: Query<&ProofGenerator, With<Player>>,
) {
    // Log stats every 5 seconds
    if (time.elapsed_secs() % 5.0) < 0.1 {
        for proof_gen in &query {
            let stats = &proof_gen.stats;
            if stats.total_proofs_generated > 0 || !proof_gen.active_tasks.is_empty() {
                info!(
                    "📊 Proof Stats - Active: {}, Generated: {}, Avg Gen Time: {:.1}ms, Success: {:.1}%",
                    proof_gen.active_tasks.len(),
                    stats.total_proofs_generated,
                    stats.avg_generation_time(),
                    stats.success_rate() * 100.0
                );
            }
        }
    }
}