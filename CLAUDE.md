# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Fowl Engine** (BFNEXT) is a dynamic campaign system for DCS World, built in Rust. The project enables persistent, multiplayer warfare campaigns with territory control, logistics, objectives, and various combat systems.

- **Discord**: https://discord.gg/wAsBEfse
- **Presentation**: https://docs.google.com/presentation/d/1EAOe0iK-1s6i0UV5ObxSD86gGBj1Ixz6FOotQn5XPdc/edit#slide=id.g2b6a346170f_1_35
- **Test Server**: The Coop - Operation Fowl Intent

## Build Setup

### Prerequisites

Before building, you must configure environment variables to link against DCS's Lua runtime:

**Linux/macOS**:
```bash
source setup-build.sh
```

**Windows (PowerShell)**:
```powershell
. .\setup-build.ps1
```

This sets:
- `LUA_LIB`: Path to lua.dll/lua.lib
- `LUA_LINK`: "dylib" (dynamic linking)
- `LUA_LIB_NAME`: "lua"

**Important**: The project must link to the same Lua version as DCS (currently Lua 5.1). Dynamic linking ensures compatibility. When DCS updates, copy the new `lua.dll` from DCS's `bin-mt` folder and regenerate `lua.lib` using `dll2lib.bat` (Windows SDK required).

### Building

**Primary build target** (DCS mission script DLL):
```bash
cargo build --release --package=bflib
```

Output: `target/release/bflib.dll` (or `.so` on Linux)

**Build all workspace members**:
```bash
cargo build --release
```

**Fast iteration builds**: The commented-out profile in root `Cargo.toml` provides faster builds for development, though mlua requires release mode for proper linking.

**Build bftools** (mission file generator):
```bash
cd bftools
cargo build --release
```

**Build user guide** (MDBook):
```bash
cd user-guide
mdbook build
mdbook serve  # View at http://localhost:3000
```

### Testing

The project has limited test coverage. Available tests:

```bash
# Run bfdb test binary
cargo run --bin test --package=bfdb
```

## Workspace Architecture

This is a Cargo workspace with 5 main crates:

### 1. **dcso3** - DCS Lua API Bindings
- **Purpose**: Minimal, safe Rust binding to DCS's Lua scripting API
- **Type**: Library (published crate)
- **License**: MIT
- **Key concept**: Provides direct translation of DCS Lua API with Rust safety features
- **Location**: `dcso3/src/`
- **Modules**:
  - `env/`: Mission environment and miz file handling
  - `event.rs`: DCS event system
  - `hooks.rs`: DCS hook system for mission scripts
  - `net.rs`: Multiplayer networking APIs
  - `controller.rs`, `group.rs`, `unit.rs`: Unit/group control
  - `coalition.rs`, `country.rs`: Faction management
  - `trigger.rs`, `timer.rs`: Mission scripting utilities
  - `world.rs`, `land.rs`: World queries
  - `mission_commands.rs`: F10 radio menu system

### 2. **bflib** - Campaign Mission Script
- **Purpose**: Main campaign logic, loaded as DLL into DCS Lua environment
- **Type**: cdylib (shared library)
- **License**: AGPL v3
- **Build output**: DLL loaded via Lua `require()` statement in DCS mission
- **Location**: `bflib/src/`
- **Key modules**:
  - `lib.rs`: Entry point, event handlers, main game loop
  - `db/`: Campaign state management
    - `player.rs`: Player registration, slot authorization, lives
    - `objective.rs`: Objectives and territory control
    - `group.rs`: Unit spawning and management
    - `logistics.rs`: Supply and logistics system
    - `cargo.rs`: Cargo transport mechanics
    - `actions.rs`: Player-triggered actions (deployments, missions)
  - `menu/`: F10 radio menu implementations
    - `action.rs`: Actions menu (deploy units, call missions)
    - `jtac.rs`: JTAC targeting system
    - `cargo.rs`: Cargo transport menu
    - `troop.rs`: Troop movement menu
    - `ewr.rs`: Early Warning Radar reports
  - `bg/`: Background tasks (async runtime integration)
    - `net.rs`: Netidx networking
    - `logpub.rs`: Log publishing
    - `statspub.rs`: Stats publishing
    - `rpcs.rs`: RPC handlers
  - `jtac.rs`: Joint Terminal Attack Controller system
  - `chatcmd.rs`: Chat command processing
  - `admin.rs`: Admin commands
  - `spawnctx.rs`: Unit spawn context management
  - `shots.rs`: Shot/kill tracking
  - `ewr.rs`: EWR implementation
  - `msgq.rs`: Message queue system

### 3. **bfdb** - Database & Web Server
- **Purpose**: Stats database, web interface, and persistence layer
- **Type**: Binary (with multiple entry points)
- **License**: N/A (internal)
- **Location**: `bfdb/src/`
- **Binaries**:
  - `bfdb`: Main web server for stats and campaign UI
  - `test`: Database testing utility
  - `migrate`: Database migration tool
- **Key files**:
  - `db.rs`: Main database implementation using Sled
  - `db_id.rs`: Database ID types

### 4. **bfprotocols** - Shared Protocol Types
- **Purpose**: Common data structures and protocols shared across crates
- **Type**: Library
- **Location**: `bfprotocols/src/`
- **Contents**:
  - `cfg/`: Campaign configuration types
  - `db/`: Database schemas for objectives, groups
  - `perf.rs`: Performance monitoring types
  - `stats.rs`: Statistics types
  - `shots.rs`: Shot tracking types

### 5. **yats** - Yet Another Typed Sled
- **Purpose**: Type-safe wrapper around Sled embedded database
- **Type**: Library
- **Location**: `yats/src/`
- **Features**:
  - Big-endian number encoding for proper ordering
  - Tuple-based prefix iteration
  - Error handling via anyhow (no panics on serialization errors)

### Excluded: **bftools** - Mission File Generator
- **Purpose**: CLI tool to generate final DCS mission (.miz) files from templates
- **Type**: Binary (standalone, not in workspace)
- **Location**: `bftools/`
- **Usage**:
  ```bash
  bftools.exe miz --output final.miz --base base.miz --weapon weapons.miz --options options.miz --warehouse warehouse.miz
  ```

## Data Flow

1. **DCS Mission → bflib**: DCS loads `bflib.dll` via Lua, fires events
2. **bflib → dcso3**: Campaign logic calls DCS API through dcso3 bindings
3. **bflib ↔ bfdb**: Netidx pub/sub for stats, state synchronization
4. **bfdb → Web**: Warp web server serves campaign UI and stats
5. **All crates → bfprotocols**: Shared types for serialization/communication

## Key Technologies

- **mlua**: Lua-Rust FFI (module mode for DLL loading)
- **Netidx**: Publish/subscribe system for distributed state
- **Sled**: Embedded database (wrapped by yats)
- **Tokio**: Async runtime for background tasks
- **Warp**: Web server framework
- **nalgebra**: Vector/matrix math for 3D operations
- **Serde**: Serialization (bincode, JSON)

## Development Workflow

1. **Source environment**: Run `setup-build.sh` or `setup-build.ps1`
2. **Make changes** to relevant crate
3. **Build**: `cargo build --release --package=<crate>`
4. **For bflib changes**: Copy `target/release/bflib.dll` to DCS mission folder, test in-game
5. **For dcso3 changes**: May affect all dependent crates
6. **For bftools changes**: Rebuild standalone in `bftools/` directory

## Important Notes

- **Lua linking**: Always use release builds for bflib due to mlua's linking requirements
- **DCS compatibility**: When DCS updates, verify Lua version compatibility
- **Licenses**: dcso3 is MIT, bflib is AGPL v3
- **Netidx**: Campaign uses netidx for RPC and stats - requires netidx infrastructure
- **No traditional tests**: Limited unit testing; testing primarily done in DCS
