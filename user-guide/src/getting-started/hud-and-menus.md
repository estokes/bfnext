# Understanding the Menus

Fowl Engine provides several ways to interact with the campaign system. This page covers the essential interface elements you'll use constantly.

## Chat System

### In-Game Chat
The chat system is your primary way to issue commands and communicate.

**Opening Chat**:
- Press `Shift+Tab` to open chat
- Type your command
- Press Enter to send

**Chat Visibility**:
- Commands starting with `-` are processed by the system
- Most commands are only visible to you
- Regular chat messages are visible to your team

### Command Prefix
Almost all system commands start with a dash `-`:
```
-help
-lives
-time
-balance
```

See the [Chat Commands](../gameplay/chat-commands.md) section for a complete list.

## F10 Map Menu

The F10 map is your tactical hub in DCS, and Fowl Engine extends it with powerful new features.

### Accessing the F10 Map
1. Press `F10` to open the map
2. You'll see standard DCS map features plus Fowl Engine additions
3. Right-click to access context menus

### Map Markers
Fowl Engine adds several types of markers:

**Objective Markers**:
- Show ownership (Blue/Red)
- Display health and logistics status
- Indicate supply levels

**JTAC Markers**:
- Show active JTAC targets
- Include laser codes for precision strikes
- Updated in real-time

**Player Markers**:
- Your F10 marks become menu targets
- Used for spawning units and waypoints
- Limited to 24 characters for menu display

## F10 Radio Menu

The F10 radio menu (not to be confused with the map) provides access to coalition commands while in your aircraft.

### Main Menu Categories

Fowl Engine adds these top-level menus:

1. **Actions** - Deploy units, call support missions
2. **JTAC** - Control JTAC units for targeting
3. **Cargo** - Manage cargo loading/unloading
4. **Troops** - Load/unload infantry units
5. **EWR** - Early Warning Radar reports

Each menu is context-sensitive and only appears if:
- Your aircraft has the necessary capability
- You have the required permissions
- The system is enabled for your role

## F10 Map vs F10 Radio Menu

It's important to understand the difference:

| F10 Map | F10 Radio Menu |
|---------|----------------|
| Shows battlefield overview | In-aircraft commands |
| Always accessible | Only in slotted aircraft |
| Place markers | Execute actions |
| Strategic planning | Tactical execution |

**Common workflow**:
1. Open **F10 Map** to plan
2. Place markers at target locations
3. Slot into aircraft
4. Open **F10 Radio Menu** → Actions
5. Select action linked to your map marker

## Menu Navigation Tips

### Using Number Keys
- Each menu item has a number (1-9)
- Press the number to select instantly
- Faster than mouse navigation in combat

### Menu Structure
Menus use hierarchical structure:
```
F10
├── Actions
│   ├── Deploy AWACS (1500 pts)
│   │   └── [Your Map Markers]
│   ├── Deploy Fighters (800 pts)
│   └── Next>>
├── JTAC
│   ├── Status
│   └── Fire Mission
└── EWR
    ├── Report
    └── Toggle
```

### "Next>>" Pagination
- Long lists are split into pages
- Select "Next>>" to see more options
- Common in Actions and JTAC menus

## On-Screen Messages

Fowl Engine sends you messages through the DCS message system:

**Message Types**:
- **White text**: Information and confirmations
- **Command responses**: Results of your actions
- **System announcements**: Team-wide notifications

**Message Duration**:
- Most messages display for 5-10 seconds
- Critical messages may persist longer
- Check chat log to review missed messages

## Accessibility Options

### Units System
Toggle between Imperial and Metric units:
- F10 → EWR → "Units to Imperial"
- F10 → EWR → "Units to Metric"

## Quick Reference

**Must-Know Commands**:
- `-help` - Command list
- `-lives` - Status check
- `-balance` - Check points

## See Also

- [Core Gameplay](../gameplay/objectives.md) - Campaign mechanics

