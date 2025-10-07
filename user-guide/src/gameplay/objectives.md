# Objectives

Objectives are the heart of the Fowl Engine dynamic campaign. Understanding how they work is essential for strategic success.

## What Are Objectives?

Objectives represent strategic locations on the battlefield:
- **Airbases**: Major air facilities with full services
- **FARPs**: Forward Arming and Refueling Points
- **FOBs**: Forward Operating Bases for ground operations
- **Logistics Hubs**: Supply distribution centers

## Objective Types

### Airbases
- Full repair and rearm capabilities
- Spawning locations for aircraft
- Critical for air superiority
- Highest strategic value

### FARPs (Forward Arming and Refueling Points)
- Forward helicopter bases
- Limited repair capabilities
- Mobile deployment possible
- Tactical importance

### FOBs (Forward Operating Bases)
- Ground unit staging areas
- Limited but essential support
- Strategic positions for ground war
- Supply storage

### Logistics Hubs
- Central supply distribution
- Connect to multiple objectives
- Critical for sustained operations
- Often heavily defended

## Objective Ownership

### Current Owner
Each objective is controlled by:
- **Blue Coalition**
- **Red Coalition**
- **Neutral** (rare, usually initial state)

### Ownership Display
Check ownership via:
- **F10 Map Markers**: Color-coded (Blue/Red)
- **Objective Name**: Prefix indicates owner
- **JTAC Reports**: Include ownership info

## Objective Status

### Health
Indicates physical damage to facilities:
- **100%**: Fully operational
- **75-99%**: Minor damage
- **50-74%**: Moderate damage
- **25-49%**: Heavy damage  
- **0-24%**: Critical condition

**Effects of Low Health**:
- Reduced repair speeds
- Limited aircraft spawns
- Slower logistics processing

### Logistics (Logi)
Represents infrastructure for supply operations:
- **0**: Completely destroyed, **can be captured**
- **1-100**: Infrastructure present, **cannot be captured**

**Key Rule**: An objective can only be captured when its Logi is at **0%** (destroyed).

### Supply Level
Resources available for operations:
- **100%**: Fully supplied
- **50-99%**: Adequate supplies
- **25-49%**: Low supplies
- **0-24%**: Critical shortage

### Fuel Level
Aviation fuel availability:
- **100%**: Full fuel stocks
- **0%**: No fuel available

## Objective States

### Threatened
An objective becomes "threatened" when:
- Enemy units are nearby (within aircraft-specific threat distance)
- Enemy ground forces are close
- Recently captured by enemy

**Cooldown**: 300 seconds (5 minutes) - objective stays threatened for 5 minutes after last enemy contact

**Effects**:
- ⚠️ **Blocks cargo and troop deployment** - cannot unload at threatened objectives!
- May trigger defensive unit spawns
- Tracked internally by system

**Important**: You cannot deploy units (troops or crates) at threatened objectives. The system will show: "you can't deploy troops here while enemies are near"

### Capturable
Ready to be captured when:
- Logi = 0
- Troops in capture zone
- Correct troop type present

**Visual Indicator**: Capturable objectives show a **white circle** on F10 map instead of the owner's color.

## Objective Information

## Reading Map Markers

Typical objective marker format:
```
Musa Airbase
Health: 100
Logi: 100
Supply: 99
Fuel: 100
Points: 0
```

Breakdown:
- **Objective name** - First line
- **Health**: 100 - Facility condition (0-100)
- **Logi**: 100 - Infrastructure (0-100, must be 0 to capture)
- **Supply**: 99 - Equipment stocks (0-100)
- **Fuel**: 100 - Fuel stocks (0-100)
- **Points**: 0 - Point value for capturing

**Capturable Example**:
```
Enemy Base
Health: 65
Logi: 0
Supply: 45
Fuel: 30
Points: 0
```
Note `Logi: 0` means this objective CAN be captured!

**Visual Indicator**: When an objective becomes capturable (Logi: 0), the **circle around the airbase on the F10 map turns WHITE** instead of the owner's color. This is an easy way to spot capturable objectives at a glance!

## Objective Zones

### Capture Zone
The physical area for capturing:
- Defined by mission design (circular or polygonal)
- Infantry must be inside to capture
- Check F10 map markers for zone location
- Only one zone per objective

## Next Steps

Learn how to [capture objectives](./capturing-objectives.md) and manage the [logistics system](./logistics.md).

