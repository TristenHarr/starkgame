# SafePlay - zk-STARK Anti-Cheat Game Demo

A 2D Bevy game that uses Plonky3 zk-STARK proofs to cryptographically verify player movement is legitimate.

## What It Does

- **2D Movement Game**: Arrow keys/WASD to move a blue square around
- **Cryptographic Proofs**: Every movement trace generates a zk-STARK proof with verification
- **Anti-Cheat Detection**: Built-in cheats (teleport, speed hack) that get caught by proof verification
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
- **`proof_system.rs`** - Generates and verifies zk-STARK proofs from movement traces
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

## How Anti-Cheat Works

### Debug Mode
- **Proof Generation**: Plonky3's `prove()` function checks constraints during generation
- **Proof Verification**: Additional verification step catches any invalid proofs
- **Result**: Cheating causes either proof generation failure or verification failure → Game crashes

### Release Mode  
- **Proof Generation**: Plonky3 skips constraint checking for performance
- **Proof Verification**: Critical verification step catches constraint violations
- **Result**: Invalid traces generate proofs that fail verification → Game crashes

## Building

```bash
# Debug mode - constraint checking during prove() + verification
cargo run

# Release mode - verification only (faster proof generation)
cargo run --release
```

Both modes provide complete anti-cheat protection through proof verification.

## Dependencies

- **Bevy 0.15**: Game engine
- **Plonky3**: zk-STARK proof system
- **Standard crates**: rand, serde, bincode, futures-lite

---

This demonstrates how zk-STARK proofs can provide mathematical guarantees against cheating in games, enabling trustless multiplayer without requiring players to trust each other or a central server.