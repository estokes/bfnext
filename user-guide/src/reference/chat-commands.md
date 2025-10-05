# Complete Chat Command Reference

Quick reference for all Fowl Engine chat commands.

## Player Commands

### Registration & Status

| Command | Description | Example |
|---------|-------------|---------|
| `blue` | Register for Blue team | `blue` |
| `red` | Register for Red team | `red` |
| `-switch blue` | Switch to Blue team | `-switch blue` |
| `-switch red` | Switch to Red team | `-switch red` |
| `-lives` | Check lives, points, status | `-lives` |
| `-balance` | Check point balance | `-balance` |
| `-time` | Check server restart time | `-time` |
| `-help` | Show command list | `-help` |

### Unit Management

| Command | Description | Example |
|---------|-------------|---------|
| `-bind <id>` | Bind troop for control | `-bind 12345` |
| `-delete <id>` | Delete deployed group | `-delete 12345` |

### JTAC Commands

| Command | Description | Example |
|---------|-------------|---------|
| `-jtac status` | Get JTAC status | `-jtac status` |

### Point Transfers (if enabled)

| Command | Description | Example |
|---------|-------------|---------|
| `-transfer <pts> <player>` | Transfer points to player | `-transfer 500 Viper21` |
| `-transfer <pts> objective:<name>` | Transfer points to objective | `-transfer 100 objective:Batumi` |

## Command Syntax Notes

- `<required>` = Required parameter
- `[optional]` = Optional parameter
- `<option1|option2>` = Choose one
- Case-insensitive for most commands
- Player names may be case-sensitive

## See Also

- [Chat Commands Guide](../gameplay/chat-commands.md) - Detailed explanations

