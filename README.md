# SafePlay - zk-STARK Anti-Cheat Game Demo

A 2D Bevy game that uses Plonky3 zk-STARK proofs to cryptographically verify player movement is legitimate.

## What It Does

- **2D Movement Game**: Arrow keys/WASD to move a blue square around
- **Cryptographic Proofs**: Every movement trace generates a zk-STARK proof
- **Anti-Cheat Testing**: Built-in cheats (teleport, speed hack) that get caught by the proof system
- **Real-time Stats**: Shows FPS, proof generation status, and player velocity

## Controls

- **Arrow Keys / WASD**: Move player
- **F**: Cycle FPS limits
- **Left Click**: Teleport to mouse (cheating)
- **Space**: 2x speed boost (cheating)
- **Left Shift**: 3x speed multiplier (cheating)

## Files

- **`main.rs`** - Game setup, movement physics, input handling, cheat systems
- **`movement_trace.rs`** - Collects movement data into 1-second traces
- **`movement_air.rs`** - Defines the mathematical constraints for valid movement
- **`proof_system.rs`** - Generates zk-STARK proofs from movement traces
- **`fps_display.rs`** - UI for stats and performance monitoring

## Key Implementation

### Movement Constraints

The `MovementAir` defines what constitutes valid movement:

1. **Boolean Inputs**: Direction flags must be 0 or 1
2. **Velocity Consistency**: Speed must match input × 200 pixels/second  
3. **Position Continuity**: Position changes must match velocity × time
4. **No Teleportation**: Instant position jumps are mathematically impossible

### Deterministic Physics

Uses integer math to ensure identical behavior in debug/release modes:

```rust
let delta_x = (velocity.x * 15) / 1000;  // Fixed timestep
position.x += delta_x;
```

### Trace Determinism Fix

We modified Plonky3's source code to access `check_constraints` as a diagnostic tool:
- Changed `pub(crate) fn check_constraints` to `pub fn check_constraints`
- Used it to debug non-deterministic traces between debug/release builds

## How Cheating Gets Caught

1. **Cheater modifies game** (teleport, speed hack)
2. **Invalid trace collected** (violates mathematical constraints)
3. **Proof generation attempts** to create STARK proof
4. **Mathematical inconsistency** results in either failure or invalid proof
5. **Verifier rejects** the proof (if it even gets generated)

The security comes from the mathematical properties of zk-STARKs, not from constraint checking during proof generation.

## Building

```bash
cargo run          # Debug mode
cargo run --release # Release mode
```

## Dependencies

- **Bevy 0.15**: Game engine
- **Plonky3**: zk-STARK proof system (locally modified)
- **Standard crates**: rand, serde, bincode, futures-lite

---

This demonstrates how zk-STARK proofs can provide mathematical guarantees against cheating in games, enabling trustless multiplayer without requiring players to trust each other or a central server.