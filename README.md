# SafePlay - zk-STARK Anti-Cheat Game Demo

A 2D Bevy game demonstrating cryptographic anti-cheat protection using Plonky3 zk-STARK proofs. Every player movement is mathematically proven to be legitimate, making cheating cryptographically impossible.

## 🎮 What It Does

- **2D Movement Game**: Control a blue square with arrow keys/WASD
- **Zero-Knowledge Proofs**: Every 0.1-second movement trace generates a zk-STARK proof
- **Mathematical Anti-Cheat**: Cryptographic verification catches all cheating attempts
- **Built-in Exploits**: Multiple cheat systems for testing the anti-cheat protection
- **Real-time Monitoring**: Live FPS, proof statistics, and cheat detection alerts
- **State Management**: Seamless game state transitions between playing and cheat detection

## 🎯 Controls

### Movement
- **Arrow Keys / WASD**: Move player at 200 pixels/second

### Built-in Cheats (for testing)
- **Left Click**: Teleport to mouse cursor (instant position jump)
- **Space**: 2x speed boost 
- **Left Shift**: 3x speed multiplier
- **Left Control**: Speed reduction

### Game Controls
- **ESC**: Dismiss cheat popup and reset game to origin

## 🏗️ Architecture

### Core Components

| File | Purpose |
|------|---------|
| **`main.rs`** | Game engine, physics, input handling, state management, cheat systems |
| **`movement_trace.rs`** | Collects movement data into 0.1-second traces with seamless boundaries |
| **`movement_air.rs`** | Mathematical constraints defining valid movement (AIR - Algebraic Intermediate Representation) |
| **`proof_system.rs`** | Async zk-STARK proof generation and verification using Plonky3 |
| **`fps_display.rs`** | Real-time UI showing performance metrics and proof statistics |

### Anti-Cheat System Flow

```
Player Input → Movement Physics → Trace Collection → Proof Generation → Verification → Cheat Detection
     ↓              ↓                    ↓                  ↓              ↓            ↓
  Arrow keys    Integer math        0.1s traces      zk-STARK proof    Crypto verify   Game state
```

## 🔒 Mathematical Constraints

The `MovementAir` defines four core constraints that all movement must satisfy:

### 1. **Boolean Inputs** 
Direction flags must be exactly 0 or 1 (no intermediate values)

### 2. **Velocity Consistency**
```rust
expected_velocity = (input_right - input_left) * SPEED + OFFSET
// Where SPEED = 200 pixels/second, OFFSET = 1000 (for negative handling)
```

### 3. **Position Continuity** 
```rust
next_position = current_position + next_velocity * PHYSICS_FACTOR
// Where PHYSICS_FACTOR = 15 (deterministic timestep)
```

### 4. **Origin Enforcement**
First trace after any game reset must start at position (0,0) with velocity (0,0)

## ⚡ Deterministic Physics

Uses integer-only mathematics for identical behavior across debug/release builds:

```rust
// Fixed timestep calculation - no floating point
let delta_x = (velocity.x * 15) / 1000;
position.x += delta_x;
```

- **Field Encoding**: Positions use ±50k pixel range with 50M offset for finite field compatibility
- **Velocity Encoding**: ±1k range with 1k offset to handle negative velocities
- **BabyBear Field**: 31-bit prime field (2^31 - 2^27 + 1) provides ~2 billion value range

## 🛡️ Anti-Cheat Protection

### How It Works

1. **Continuous Monitoring**: Every frame records position, velocity, and input state
2. **Trace Boundaries**: Movement data collected in 0.1-second traces with seamless transitions
3. **Matrix Generation**: Traces converted to 9-column constraint matrices
4. **Proof Generation**: zk-STARK proofs generated asynchronously in background threads
5. **Cryptographic Verification**: Every proof immediately verified for constraint satisfaction
6. **Instant Detection**: Any verification failure triggers immediate cheat detection

### Debug vs Release Mode

| Mode | Proof Generation | Verification | Anti-Cheat Coverage |
|------|------------------|--------------|-------------------|
| **Debug** | Constraint checking + proof | Full verification | Complete protection |
| **Release** | Fast generation (no checking) | Full verification | Complete protection |

Both modes provide identical anti-cheat protection through the verification step.

### Detected Cheat Types

- **🚀 Teleportation**: Instant position jumps violate position continuity
- **💨 Speed Hacking**: Velocities exceeding input×200 violate velocity consistency  
- **🎮 Input Manipulation**: Non-boolean inputs violate boolean constraints
- **⚖️ Physics Violations**: Any movement not matching physics equations fails verification
- **🎯 Origin Bypassing**: Starting anywhere except (0,0) after reset violates origin constraints

## 🚀 Building & Running

```bash
# Debug mode - Full constraint checking + verification
cargo run

# Release mode - Optimized generation + verification (recommended)
cargo run --release
```

### System Requirements
- Rust 2024 edition
- Local Plonky3 installation at `../Plonky3/`

## 📊 Real-time Monitoring

### Performance Display
- **FPS**: Color-coded frame rate (Green: 55+, Yellow: 30-55, Red: <30)
- **Proof Stats**: Active tasks, total generated, average generation/verification times
- **Velocity**: Current speed with cheat detection indicators

### Cheat Detection UI
- **Modal Alert**: Red popup when cheating detected
- **Detailed Message**: Shows verification failure reason
- **Game Reset**: ESC key returns to origin and clears all traces

## 🧬 Technical Implementation

### Cryptographic Stack
- **Field**: BabyBear (31-bit prime field for efficient operations)
- **Hash**: Poseidon2 with 16-width permutation
- **Commitment**: Merkle trees with 8-ary branching
- **PCS**: Two-adic FRI (Fast Reed-Solomon Interactive Oracle Proofs)
- **Challenge**: Binomial extension field for Fiat-Shamir transformation

### State Management
- **Playing State**: All game systems active (movement, input, proof generation)
- **CheatDetected State**: Game paused, only cheat popup and reset systems active
- **Race Condition Prevention**: State machine prevents exploitation during transitions

## 🔧 Dependencies

### Core
- **Bevy 0.15**: Modern ECS game engine
- **Plonky3**: Complete zk-STARK proof system suite
  - `p3-air`: Constraint system definition
  - `p3-uni-stark`: Proof generation and verification
  - `p3-baby-bear`: Prime field arithmetic
  - `p3-fri`: Polynomial commitment scheme
  - `p3-merkle-tree`: Cryptographic commitments

### Support
- **Async**: `futures-lite` for non-blocking proof generation
- **Serialization**: `bincode`, `serde` for proof data
- **Randomness**: `rand` for cryptographic parameters

## 🎯 Security Guarantees

This system provides **mathematical guarantees** against cheating:

- **🔐 Cryptographic Soundness**: zk-STARK proofs are computationally sound - invalid proofs cannot be generated
- **📈 Complete Coverage**: Every movement frame is recorded and proven with zero gaps
- **⚡ Immediate Detection**: Cheating detected within 0.1 seconds through proof verification
- **🛡️ Unforgeable**: Cannot generate valid proofs for invalid movements due to cryptographic properties
- **🎲 Deterministic**: Integer math ensures consistent behavior across all platforms and build modes

## 🌟 Use Cases

This demonstrates how **zero-knowledge proofs enable trustless gaming**:

- **🌐 Decentralized Multiplayer**: No central authority needed to verify fair play
- **🤝 Peer-to-Peer Trust**: Players don't need to trust each other - mathematics guarantees fairness
- **🏆 Competitive Integrity**: Cryptographic proofs provide undeniable evidence of legitimate gameplay
- **🔒 Anti-Cheat as a Service**: Proof verification can be outsourced to any third party

---

*SafePlay showcases the future of gaming where cryptographic mathematics, not trust, ensures fair play.*