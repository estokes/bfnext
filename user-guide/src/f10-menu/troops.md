# Troop Transport

Move infantry units to capture objectives, reinforce positions, and support ground operations.

## Overview

Transport infantry via helicopter or ground vehicle:
- Load troops at friendly objectives
- Move to target location
- Unload for capture or defense
- Critical for objective capture

## Requirements

**Transport Capability**:
- Helicopters: Mi-8, UH-60, CH-47, etc.
- APCs/IFVs: BTR, BMP, Bradley, etc.
- Check vehicle specifications

**Proximity**: Must be near troops/objective

## Troops Menu

Access via F10 → Troops

**Menu Options**:
- **Load**: Pick up infantry
- **Unload**: Drop off troops  
- **Status**: Check loaded troops

## Loading Troops

**Steps**:
1. Position near objective/FARP
2. F10 → Troops → Load
3. Select troop type
4. Troops board automatically
5. Confirmation message

**Troop Types**:
- Infantry squads (can capture)
- Assault troops
- Special forces
- Anti-tank teams
- Support units

**Capacity** (PG Tempest aircraft):
- **CH-47**: 4 troop slots (largest)
- **Mi-8**: 3 troop slots
- **UH-1H**: 2 troop slots
- **SA342/Mi-24**: 1 troop slot each
- Each slot = one infantry squad
- APCs vary by type

## Transporting Troops

**Flight Operations**:
- Troops count as weight
- May affect performance
- Stay low for safety
- Fast ingress/egress

**Ground Movement**:
- APCs/IFVs can transport troops overland

## Unloading Troops

**Steps**:
1. Land/stop at destination
2. **Objective must NOT be threatened** ⚠️
3. F10 → Troops → Unload
4. Select troops or "Unload All"
5. Troops dismount
6. Troops now on ground

**Critical Restriction**:
- ⚠️ Cannot unload at **threatened objectives**!
- Error: "you can't deploy troops here while enemies are near"
- Objective stays threatened for **5 minutes** after enemies leave

**Location Matters**:
- For capture: Inside capture zone (must be unthreatened)
- For defense: Strategic positions
- For assault: Covered approaches

## Troop Management

### After Unloading

**Troop Control**:
- Troops become deployed group
- Assigned group ID
- Control via commands or F10

**Movement**:
```
-bind <troop-id>
```
Then use F10 → Actions → Move (costs 15 points)

**Deletion**:
```
-delete <troop-id>
```
Gives 50% refund

## See Also

- [Deployable Units Reference](../reference/deployables.md) - Troop types and costs
- [Capturing Objectives](../gameplay/capturing-objectives.md)
- [Cargo Operations](./cargo.md)

