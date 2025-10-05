# Actions Menu

The Actions menu lets you deploy units, call support missions, and execute strategic operations using your earned points.

## Overview

Actions are point-based deployments that let you:
- Deploy support aircraft (AWACS, tankers)
- Call fighter packages (CAP, SEAD, strike)
- Spawn drones and missiles
- Deploy ground units
- Execute logistics operations

## Accessing Actions

1. Open F10 radio menu
2. Select "Actions>>" (usually option 1)
3. Menu expands showing available actions

**Requirements**:
- Must be slotted in aircraft
- Actions menu enabled for your role
- Sufficient points for desired action

## Action Categories

### Support Aircraft

**AWACS (Airborne Warning and Control)**:
- Cost: **50 points** (PG Tempest)
- Provides radar coverage (400km)
- Extends team awareness
- Place marker for orbit location
- RTB refunds 25% (13 points)

**Tanker Aircraft**:
- Check server for cost
- Aerial refueling support
- Extends mission duration
- Position along likely routes
- RTB refunds 25%

### Fighter Packages

**CAP (Combat Air Patrol)**:
- Cost: **200 points** (PG Tempest)
- Deploys fighters to area
- Defends against enemy air
- Place marker at patrol location
- RTB refunds 25% (50 points)

**SEAD (Suppression of Enemy Air Defenses)**:
- Cost: **200 points** (PG Tempest)
- Attacks SAM sites
- Clears path for strikes
- Target high-threat areas
- RTB refunds 25% (50 points)

**Attack Helicopters**:
- Cost: **200-300 points** (PG Tempest)
- Ground attack missions
- Close air support
- RTB refunds 25%

### Drones & Missiles

**Reconnaissance Drone**:
- Small: **50 points** (12km JTAC range)
- Large: **100 points** (18km JTAC range)
- Surveillance over area
- JTAC capability included
- Cannot RTB for refund (stay deployed)

**Cruise Missile Platforms**:
- S-3B: **25 points**
- Tu-95/Tu-160: **150 points**
- Long-range precision strikes
- High-value target elimination
- RTB refunds 25%

### Ground Deployables

**Ground Deployables** (via cargo crates):
- See [Deployable Units Reference](../reference/deployables.md) for complete list
- SAMs: 5-100 points (1-4 crates)
- Tanks: 65-80 points (3 crates)
- IFVs: 20-45 points (2 crates)
- Artillery: 50-100 points (2-3 crates)
- Can delete for **50% refund**

## Using Actions

### Basic Workflow

1. **Plan**: Decide what to deploy
2. **Mark**: Place F10 map marker at target location
3. **Check**: Verify point balance
4. **Deploy**: F10 → Actions → [Action] → [Marker]
5. **Confirm**: Action spawns, points deducted

### Detailed Steps

**Step 1: Place Map Marker**
- Open F10 map (not radio menu!)
- Right-click target location
- Select "Add Mark..."
- Enter short name (≤24 characters)
- Examples: "AWACS1", "CAP", "SAM"

**Step 2: Open Actions Menu**
- In aircraft, press F10
- Select "Actions>>"
- Browse available actions

**Step 3: Select Action**
- Each action shows cost: "Deploy CAP (800 pts)"
- Insufficient points = grayed out
- Select desired action

**Step 4: Choose Location**
- Your map markers appear as options
- Select appropriate marker
- For waypoint actions, see grouped units

**Step 5: Confirmation**
- Unit spawns at location
- Points deducted immediately
- System message confirms
- Unit appears on F10 map

## Advanced Actions

### Waypoint Actions

Control deployed units:
- **Tanker Waypoint**: Move tanker to new orbit
- **AWACS Waypoint**: Reposition AWACS
- **Fighter Waypoint**: Send fighters to new CAP location
- **RTB (Return to Base)**: Recall unit

**How to Use**:
1. Deploy initial unit (e.g., AWACS)
2. Place new marker for waypoint
3. F10 → Actions → [Unit Type] Waypoint
4. Select unit from list
5. Select new waypoint marker
6. Unit moves to location

### Move Action

Relocate deployed ground units:
- Tanks, SAMs, artillery, infantry
- Place destination marker
- F10 → Actions → Move
- Select unit
- Select destination
- Unit moves

### Logistics Actions

**Logistics Repair**:
- Repairs objective logistics
- Select from owned objectives
- Costs points (200-500)
- Immediate logi increase

**Logistics Transfer**:
- Admin-only usually
- Moves supplies between objectives
- Strategic rebalancing

## Action Costs (PG Tempest)

**Air Deployments**:
- AWACS: **50 pts** (RTB refund: 13 pts)
- CAP Fighters: **200 pts** (RTB refund: 50 pts)
- SEAD: **200 pts** (RTB refund: 50 pts)
- Attack Helicopters: **200-300 pts** (RTB refund: 50-75 pts)
- Bomber: **200 pts**
- Small Drone: **50 pts**
- Large Drone: **100 pts**
- ALCM Platform: **25-150 pts** (RTB refund: 6-38 pts)

**Ground Deployables** (via cargo):
- See [Deployable Units Reference](../reference/deployables.md)
- Delete refund: **50%** of cost

**Special Actions**:
- Paratroops: **100 pts**
- Logistics Repair: **100-200 pts**
- Logistics Transfer: **100 pts**

## Managing Deployed Units

### Via Chat Commands

`-action <group-id> <command>`: Control unit
`-bind <group-id>`: Bind unit for control
`-delete <group-id>`: Remove unit

### Via F10 Waypoint Actions

- Move support aircraft
- Reposition CAP
- Redirect drones
- RTB when done

### Unit Lifespan

- Units persist until destroyed
- No automatic despawn
- Delete deployables for **50% refund**: `-delete <group-id>`
- RTB aircraft for **25% refund** (via F10 menu or chat)
- Action units (AWACS, fighters) cannot be deleted by players

## See Also

- [Points System](../gameplay/points-and-lives.md) - Earning and spending

