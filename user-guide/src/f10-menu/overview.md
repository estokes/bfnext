# F10 Menu Overview

The F10 radio menu is your tactical command center in Fowl Engine. This page provides an overview of all available menus.

## Accessing F10 Menus

**In Aircraft**:
1. Slot into an aircraft
2. Press `F10` (or your configured radio menu key)
3. Navigate using number keys or mouse

**Requirements**:
- Must be in a slotted aircraft (not spectator)
- Menus vary by aircraft type
- Some menus require specific capabilities

## Main Menu Structure

When you open F10, you'll see Fowl Engine menus:

```
F10 Radio Menu
├── Actions          (Deploy units, call missions)
├── JTAC            (Control targeting units)
├── Cargo           (Load/unload cargo)
├── Troops          (Load/unload infantry)
└── EWR             (Radar reports)
```

**Note**: Not all menus appear for every aircraft. Availability depends on:
- Aircraft type and capabilities
- Server configuration
- Your permissions
- Current game state

## Menu Types

### Actions Menu
**Purpose**: Deploy units and call support missions

**What You Can Do**:
- Deploy AWACS, tankers, CAP fighters
- Call SEAD and strike missions
- Deploy drones and cruise missiles
- Spawn deployable units
- Order logistics operations

**Costs**: Most actions cost points

**See**: [Actions Menu](./actions.md) for details

---

### JTAC Menu  
**Purpose**: Control Joint Terminal Attack Controllers

**What You Can Do**:
- Check JTAC status
- Shift laser to next target
- Change laser codes
- Request fire missions (artillery/missiles)
- Mark targets with smoke
- Configure targeting filters

**Costs**: Fire missions may cost points

**See**: [JTAC System](./jtac.md) for details

---

### Cargo Menu
**Purpose**: Transport equipment and supplies

**What You Can Do**:
- Load cargo crates
- Unload cargo at objectives
- Check cargo capacity
- View loaded cargo

**Requirements**: 
- Aircraft with cargo capability
- Proximity to cargo/objective

**See**: [Cargo Operations](./cargo.md) for details

---

### Troops Menu
**Purpose**: Transport infantry units

**What You Can Do**:
- Load infantry squads
- Unload troops at objectives
- Check troop capacity
- View loaded troops

**Requirements**:
- Aircraft with troop capability
- Proximity to troops/objective

**See**: [Troop Transport](./troops.md) for details

---

### EWR Menu
**Purpose**: Early Warning Radar information

**What You Can Do**:
- Request radar reports
- Toggle EWR on/off
- Get friendly unit reports
- Change unit systems (Imperial/Metric)

**Costs**: Free

**See**: [Early Warning Radar](./ewr.md) for details

---

## Menu Navigation

### Number Keys
- Each menu item has a number (1-9)
- Press number to select
- Press `0` or `Escape` to go back

### Pagination
- Long lists split into pages with "Next>>"
- Select "Next>>" to see more options

## Context-Sensitive Menus

Menus adapt to your situation:

**Aircraft Type**:
- Cargo menu only in cargo aircraft
- Troop menu only in troop transports
- Different options per aircraft

**Location**:
- Proximity affects available actions
- Some menus require being near objectives
- Landing/ground vs. airborne options differ

**Game State**:
- Deployed units affect menu options
- Your point balance limits choices
- Team situation changes availability

## F10 Map Markers Integration

Many F10 menu actions use your map markers:

**How It Works**:
1. Place F10 map marker at target location
2. Name marker (≤24 characters)
3. Open F10 radio menu
4. Select action
5. Your markers appear as destination options

**Best Practices**:
- Use short, clear names
- "CAS1", "SEAD", "CAP" work well
- Delete old markers
- One marker per name (duplicates won't show)

**Example Workflow**:
1. Open F10 map
2. Right-click target area
3. Add mark "SEAD1"
4. Return to aircraft
5. F10 → Actions → Deploy SEAD → "SEAD1"
6. Mission launched!

## Menu Permissions

Some menus require permissions:

**Standard Players**:
- EWR (always available)
- Cargo (if in capable aircraft)
- Troops (if in capable aircraft)

**With Points**:
- Actions (need points to deploy)
- JTAC fire missions (may cost points)

**Server Configuration**:
- Admins can enable/disable menus
- Some servers restrict certain features
- Check server rules

## Menu Costs

Many menu actions cost points:

**Free Actions**:
- EWR reports
- JTAC status checks
- Cargo operations (usually)
- Troop transport (usually)

**Point Costs**:
- Deploying units (50-2000+ points)
- Fire missions (varies)
- Special actions (varies)

**Check Before Acting**:
- Menu shows costs: "Deploy AWACS (1500 pts)"
- Insufficient points = action unavailable
- No refunds on accidental deployments!

## Common Menu Patterns

### Deploy Action Pattern
```
F10 → Actions → [Action Name] → [Your Marker] → Confirm
```

### JTAC Fire Mission Pattern
```
F10 → JTAC → [JTAC ID] → Fire Mission → [Artillery Group] → [Rounds]
```

### Cargo Pattern
```
F10 → Cargo → Load/Unload → [Cargo Type] → Confirm
```

## Troubleshooting

### "Menu not appearing"
**Possible causes**:
- Wrong aircraft type
- Too far from required location
- Server disabled feature
- Not slotted in aircraft

**Solutions**:
- Check aircraft capabilities
- Move closer to objective
- Verify server features
- Ensure properly slotted

### "Option grayed out"
**Possible causes**:
- Insufficient points
- No valid map markers
- Action not available
- Server restrictions

**Solutions**:
- Check point balance
- Place appropriate markers
- Verify conditions met
- Ask team/admin

### "Action failed"
**Possible causes**:
- Invalid target location
- Insufficient resources
- Conflict with existing unit
- Server lag/error

**Solutions**:
- Choose different location
- Wait for resources
- Delete conflicting units
- Report persistent issues

## See Also

- [Actions Menu](./actions.md) - Deploy units and missions
- [JTAC System](./jtac.md) - Advanced targeting
- [Cargo Operations](./cargo.md) - Transport supplies
- [Troop Transport](./troops.md) - Move infantry
- [EWR Reports](./ewr.md) - Radar information

