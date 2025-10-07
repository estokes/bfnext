# JTAC System

The Joint Terminal Attack Controller (JTAC) system provides advanced targeting, fire coordination, and battlefield intelligence.

## What is JTAC?

JTACs are specialized units that:
- Detect and track enemy units
- Designate targets with laser
- Coordinate artillery and missile strikes
- Provide detailed target information

## JTAC Types

### Drone JTAC (Large - MQ-9 Reaper)
- **Range**: **18 km** (18,000m)
- **Line-of-sight**: Not required (can see through terrain)
- **Cost**: 100 points to deploy
- **Duration**: 12 hours
- **Function**: Long-range surveillance and targeting

### Drone JTAC (Small)
- **Range**: **12 km** (12,000m)
- **Line-of-sight**: Not required (can see through terrain)
- **Cost**: 50 points to deploy
- **Duration**: 12 hours
- **Function**: Reconnaissance and targeting

### Ground JTAC
- Infantry or vehicle-based units with JTAC capability
- 360° coverage
- May require line-of-sight (check unit type)
- Range varies by unit type

### Player JTAC
- You can act as JTAC in certain aircraft
- Access via F10 menu
- Control your own targeting
- Range depends on your aircraft sensors

## Accessing JTAC

**F10 Menu**:
1. Press F10
2. Select "JTAC"
3. Choose JTAC unit from list
4. Access that JTAC's functions

**Format**: JTACs listed by ID (e.g., "JTAC 12345")

## JTAC Status

### Checking Status

```
F10 → JTAC → [JTAC ID] → Status
```

**Status Report Shows**:
- JTAC position (bearing/distance from objective)
- Current target (if any)
- Laser code
- Visual contacts
- Nearby artillery units
- Nearby cruise missile units
- Autoshift setting
- IR pointer setting
- Filter settings

### Reading Status

Example:
```
JTAC 12345 status
lasing T-72B code 1688 marker M123
position bearing 045 for 5.2km from Batumi

Visual On: T-72Bx3, BMP-3x2, SA-13

autoshift: true, ir_pointer: false
filter: [Tank, APC]
available artillery: [54321]
available ALCM: [65432(4)]
```

## Target Management

### Shifting Targets

**Manual Shift**:
```
F10 → JTAC → [JTAC ID] → Shift Target
```
Moves laser to next detected target.

**Auto-Shift**:
```
F10 → JTAC → [JTAC ID] → Toggle Auto-Shift
```
Automatically cycles through targets.

### Target Priority

JTACs prioritize by:
1. Unit type (configurable)
2. Threat level
3. Distance
4. Last movement

### Target Filters

```
F10 → JTAC → [JTAC ID] → Filter → [Unit Type]
```

**Filter Options**:
- Tank
- APC/IFV
- Artillery
- SAM
- Helicopter
- Infantry

**Clear Filter**:
```
F10 → JTAC → [JTAC ID] → Clear Filter
```

## Laser Designation

### Laser Codes

JTACs designate targets with laser codes:
- Default: Usually 1688
- Change via F10 menu
- Must match your weapon settings

**Changing Code**:
```
F10 → JTAC → [JTAC ID] → Code → [Hundreds/Tens/Ones]
```

Example: To change from 1688 to 1511:
1. Select "1" (changes thousands to 1000)
2. Select "500" (changes hundreds to 1500)  
3. Select "11" (changes to 1511)

**Important**: Set your LGB/missile code to match!

### Using Laser Designation

**Steps**:
1. Check JTAC status for code
2. Set weapon laser code to match
3. JTAC must be lasing target
4. Attack target with LGBs/Mavericks
5. Guide weapon to impact

**Tips**:
- Weapon must "see" laser
- Stay within parameters
- Don't break laser lock
- Multiple aircraft can use same code

### IR Pointer

```
F10 → JTAC → [JTAC ID] → Toggle IR Pointer
```

Adds infrared pointer:
- Visible in night vision
- Helps locate target
- Doesn't affect laser

## Fire Missions

### Artillery Missions

**Request Fire Mission**:
```
F10 → JTAC → [JTAC ID] → Artillery → [Battery ID] → [Rounds]
```

**Process**:
1. JTAC must have target
2. Artillery must be in range
3. Select rounds (1, 3, 5, etc.)
4. Battery fires on target

**Round Options**:
- Usually 1, 3, 5, 10, or "All"
- Check ammunition available
- Battery listed with ammo count

**Adjustments**:
```
F10 → JTAC → [JTAC ID] → Artillery → [Battery] → Adjust Fire
```

Options:
- Short/Long (range adjustment)
- Left/Right (lateral adjustment)
- Typically 50-100m increments

### Cruise Missile Missions

**Request ALCM Strike**:
```
F10 → JTAC → [JTAC ID] → ALCM → [Unit ID] → [Settings]
```

**Parameters**:
- Missiles per target
- Magazine expenditure
- Targets multiple contacts

**Cost**: May cost points

**See**: [ALCM Guide](../advanced/alcm.md) for details

### Smoke Marker

```
F10 → JTAC → [JTAC ID] → Smoke Target
```

**Creates smoke at target**:
- Visual marking
- Helps locate target
- 60-second cooldown
- Color varies by team

## Common Issues

### "No JTAC available"
- No JTAC units deployed
- JTACs killed
- Out of detection range
- Check F10 JTAC menu

### "No target"
- JTAC hasn't detected enemies
- Enemies out of range
- Line-of-sight blocked
- Wait for detection

### "Artillery out of range"
- Battery too far from target (max range: 300km)
- Deploy closer artillery
- Use different battery
- Check JTAC status for available units

### "ALCM out of range"
- Cruise missile platform too far from target (max range: 300km)
- Reposition platform using waypoint commands
- Deploy new platform closer
- Check JTAC status for available ALCM units

### "Laser not tracking"
- Wrong laser code
- Out of laser parameters
- Target moved
- JTAC lost line-of-sight

## See Also

- [Artillery Operations](../advanced/artillery.md) - Fire mission details
- [ALCM Operations](../advanced/alcm.md) - Cruise missile strikes

