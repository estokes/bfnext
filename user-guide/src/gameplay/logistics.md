# Logistics & Supply

The logistics system adds strategic depth to Fowl Engine. Understanding supply flows is key to sustained operations.

## Overview

Logistics simulates the supply chain required to maintain military operations. Without adequate supplies, objectives cannot function effectively.

## The Logistics System

### What is Logistics?

**Logistics tracks two main resources**:
1. **Equipment** (weapons, vehicles, parts)
2. **Fuel** (aviation fuel, vehicle fuel)

**Three levels of infrastructure**:
1. **Logistics (Logi)** - Infrastructure health
2. **Supply** - Equipment inventory level
3. **Fuel** - Fuel inventory level

## Supply Flow

### Logistics Hubs

**Central Distribution**:
- Logistics hubs are special objectives
- They distribute supplies to connected objectives
- Form the backbone of supply network

**Hub Connections**:
- Each hub connects to multiple objectives
- Supply flows automatically
- Captured objectives reconnect supply lines

### Supply Routes

Supplies flow from:
```
Logistics Hub → Frontline Objectives → FARPs
```

**Route Characteristics**:
- Automatic distribution every tick interval
- Prioritizes objectives with lowest supply
- Distance affects delivery amount
- Broken routes halt supply flow

### Supply Ticks

The system runs on a **tick cycle**:

**Tick Frequency** (PG Tempest):
- Every **10 minutes**
- Supplies distribute during each tick
- Automatic process, no player action needed
- Full delivery cycle: **24 ticks** (4 hours)

**During Each Tick**:
1. System assesses all objectives
2. Calculates supply needs
3. Distributes from logistics hubs
4. Updates objective supply levels

## Supply & Fuel Levels

### Supply Percentage

Represents equipment and munitions:

- **100%**: Fully stocked
- **75-99%**: Good condition
- **50-74%**: Adequate supplies
- **25-49%**: Low stocks
- **0-24%**: Critical shortage

**Effects of Low Supply**:
- Reduced repair speeds
- Limited deployments available
- Decreased operational tempo
- Warehouse capacity reduced

### Fuel Percentage

Represents aviation fuel stocks:

- **100%**: Full fuel reserves
- **75-99%**: Good fuel stocks
- **50-74%**: Adequate fuel
- **25-49%**: Low fuel
- **0-24%**: Fuel emergency

**Effects of Low Fuel**:
- Aircraft cannot rearm
- Helicopter operations limited
- May prevent takeoffs
- Logistics vehicles affected

## Supply Priorities

### Automatic Distribution

The system prioritizes:
1. **Lowest supply first** - Most desperate get priority
2. **Connected objectives** - Must have supply route
3. **Available inventory** - Hub must have supplies

**Example Priority**:
```
Objective A: 20% supply → Gets first priority
Objective B: 45% supply → Gets second priority  
Objective C: 80% supply → Gets last priority
```

## Logistics Infrastructure (Logi)

### What is Logi?

Logi represents the physical infrastructure:
- Buildings
- Roads and railways
- Communications
- Support facilities

### Logi Percentage

- **100%**: Perfect condition
- **75-99%**: Minor damage
- **50-74%**: Moderate damage
- **25-49%**: Heavy damage
- **1-24%**: Critical damage
- **0%**: Destroyed (objective capturable!)

### Logi Effects

**High Logi (75-100%)**:
- Fast repair times
- Efficient supply processing
- Normal operations

**Medium Logi (25-74%)**:
- Slower operations
- Reduced efficiency
- Still functional

**Low Logi (1-24%)**:
- Severely impaired
- Very slow repairs
- Minimal functionality

**Zero Logi (0%)**:
- **OBJECTIVE CAN BE CAPTURED**
- No supply processing
- No repairs possible
- Critical vulnerability

## Repairing Logistics

### Natural Repair

Logistics gradually repair over time:
- Automatic process
- Slow regeneration
- Requires some supply availability

**Repair Rate**:
- Typically 1-5% per tick
- Server-configured
- Requires positive supply level

### Manual Repair

Players can expedite repairs:

**Via Actions Menu**:
```
F10 → Actions → Repair (or Repair-Fast) → [Select Objective]
```

**Requirements**:
- Costs **100 points** (helo) or **200 points** (fast fixed-wing)
- Must own the objective
- Repair crate must be delivered to objective

**Benefits**:
- Immediate logi increase (one step)
- Prevents capture vulnerability
- Strategic investment
- Fast option gets there quicker

## Warehouse System

### Equipment Inventory

Objectives store equipment in warehouses:

**Types of Equipment**:
- Aircraft and helicopters
- Tanks and armored vehicles
- Artillery systems
- Infantry units
- Support equipment

**Capacity**:
- Each objective has maximum capacity
- Varies by objective type
- Display format: `stored / capacity`

### Liquid Inventory

Fuel stored separately:

**Liquid Types**:
- Jet fuel (aircraft)
- Aviation gasoline (props)
- Diesel (vehicles)

## Supply Strategies

### Offensive Strategy

**Attacking Enemy Supply**:
1. **Target logistics hubs** - Cut off multiple objectives
2. **Interdict supply routes** - Attack connecting objectives
3. **Reduce frontline supply** - Weaken enemy operations

**Maintaining Your Supply**:
1. **Protect logistics hubs** - Heavy air defense
2. **Secure supply routes** - Defend connecting objectives
3. **Keep logi above 0%** - Prevents captures

### Defensive Strategy

**Supply Line Defense**:
- Deploy SAMs at logistics hubs
- Maintain CAP over critical objectives
- Repair logi quickly when damaged
- Keep fuel reserves high

**Emergency Response**:
- If logi falls to 0%, immediate priority repair
- Rush fighters to defend against capture
- Deploy ground units to contest zone

## Reading Supply Information

### F10 Map Markers

Typical format:
```
Musa Airbase
Health: 85
Logi: 42
Supply: 75
Fuel: 100
Points: 0
```

- **Health**: 85 - Facility condition
- **Logi**: 42 - Infrastructure (safe from capture, above 0)
- **Supply**: 75 - Equipment stocks (good level)
- **Fuel**: 100 - Fuel stocks (full)
- **Points**: 0 - Capture point value

**Note**: Values are whole numbers 0-100.

### In-Game Notifications

System messages for supply events:
- "Objective supply critical" - Below 25%
- "Objective fuel emergency" - Below 25%
- "Logistics damaged" - Logi falling
- "Objective capturable" - Logi at 0%

## Logistics Transfers

### Manual Transfers

Admin or special actions can transfer supplies:

**Command**:
```
-admin transfer <from-objective> <to-objective>
```

**Use Cases**:
- Emergency supply to starved objective
- Balancing supply distribution
- Preparing for major operations

**Restrictions**:
- Requires admin privileges (for `-admin` variant)
- Limited by warehouse capacity
- Both objectives must be owned

## Advanced Topics

### Supply Line Optimization

**Efficient Network**:
- Capture objectives in logical order
- Maintain control of connecting objectives
- Don't overextend supply lines

**Example Bad Strategy**:
```
Hub → A → (enemy) → B → Front
```
Objective B is cut off!

**Example Good Strategy**:
```
Hub → A → B → Front
```
Clear supply line maintained.

### Logistics as Weapon

**Starve Enemy Objectives**:
1. Identify their logistics hubs
2. Strike them repeatedly
3. Target connecting objectives
4. Wait for supply depletion
5. Attack when weakened

**Siege Warfare**:
- Surround enemy objective
- Cut off supply routes
- Wait for supplies to deplete
- Capture when logistics fail

### Supply Consumption

Different operations consume supplies:

**High Consumption**:
- Repairing damaged aircraft
- Deploying heavy armor
- Sustained combat operations
- Large-scale actions

**Low Consumption**:
- CAP flights
- Basic repairs
- Small unit deployments

## Troubleshooting

### "Why is my objective low on supply?"

Possible causes:
- Logistics hub captured by enemy
- Supply route broken
- High consumption rate
- Insufficient tick intervals passed

**Solution**:
- Check supply route integrity
- Protect logistics hubs
- Wait for next supply tick
- Reduce unnecessary deployments

### "Logi won't repair"

Possible causes:
- Supply level too low
- Recent damage faster than repair
- Server settings

**Solution**:
- Wait for supply delivery
- Use manual repair action
- Defend objective from attacks

## Next Steps

Learn about the [Points and Lives System](./points-and-lives.md) to understand how resources are earned and used!

