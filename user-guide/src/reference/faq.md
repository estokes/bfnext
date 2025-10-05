# Frequently Asked Questions

Common questions and answers about Fowl Engine.

## Getting Started

### Q: How do I join the server?
**A**: Connect to "The Coop - Operation Fowl Intent" server, register for a team (type `blue` or `red` in chat), then select an aircraft slot.

### Q: Can I change teams?
**A**: Some servers allow limited side switching with `-switch blue` or `-switch red`. Check your remaining switches with `-lives`.

### Q: Why can't I occupy any slots?
**A**: You must register for a team first by typing `blue` or `red` in chat while in spectator mode.

## Gameplay

### Q: How do I capture an objective?
**A**: 
1. Reduce objective logistics to 0%
2. Transport capture-capable infantry to the objective
3. Unload troops inside the capture zone
4. Wait for capture to complete

See [Capturing Objectives](../gameplay/capturing-objectives.md) for details.

### Q: Why can't I capture this objective?
**A**: Check these requirements:
- Logistics must be exactly 0%
- You must have infantry troops IN the zone (unloaded, on ground)
- Troops must be capture-capable type
- No enemy troops contesting the zone

### Q: How do I earn points?
**A**: 
- Air kills: **25 points** (+5 bonus for LR SAMs)
- Ground kills: **2 points**
- Objective captures: **50 points** (split among participants)
- Starting bonus: **190 points**

See [Points System](../gameplay/points-and-lives.md) for full details.

### Q: What happens when I run out of lives?
**A**: You're restricted to spectator mode. Contact an admin to request a life reset.

## F10 Menus

### Q: I don't see the Actions menu!
**A**: Check:
- Are you slotted in an aircraft?
- Does your aircraft have permission?
- Is Actions enabled on this server?
- Try re-slotting

### Q: My map markers don't show up in Actions menu
**A**: 
- Marker names must be ≤24 characters
- Delete duplicate marker names
- Only one marker per name
- Must be YOUR markers (not others')

### Q: How do I use JTAC?
**A**: 
1. F10 → JTAC → [Select JTAC]
2. Check Status to see current target
3. Set your weapon laser code to match JTAC's code
4. Attack the lased target

See [JTAC System](../f10-menu/jtac.md) for full guide.

## Logistics & Supply

### Q: What does "Logi: 0" mean?
**A**: Logistics at 0 means the objective CAN be captured. The infrastructure is completely destroyed.

### Q: How do I repair logistics?
**A**: Logistics repair slowly over time automatically. Admins can speed this with `-admin repair`. Some servers allow player logistics repair actions.

### Q: Why is my objective low on supply?
**A**: 
- Supply routes may be broken
- Logistics hub may be captured
- High consumption from operations
- Wait for next logistics tick

### Q: How often does supply update?
**A**: Typically every 15-30 minutes (server-configured). Admins can force with `-admin tick`.

## Technical Issues

### Q: I can't load cargo/troops!
**A**: Check:
- Are you close enough to objective? (within ~50m)
- Is it a friendly objective?
- Does your aircraft have that capability?
- Is cargo/troops available?
- Try landing closer

### Q: F10 menu not responding
**A**: 
- Wait a moment (server lag)
- Try re-opening (press F10 again)
- Re-slot if persistent
- Report to admin if continues

### Q: My deployed unit disappeared!
**A**: 
- It may have been destroyed
- Check F10 map for its marker
- Server restart can remove some units
- Contact admin if it seems like a bug

## Points & Economy

### Q: How much do actions cost?
**A**: (PG Tempest values):
- AWACS: 50 points
- CAP/SEAD: 200 points each
- Drones: 50-100 points
- Ground deployables: 5-500 points (see Deployables Reference)

Check the menu - costs are shown in the action name.

### Q: Can I get refunds?
**A**: Yes! Use `-delete <group-id>` to delete your deployed units and get **50% of the cost back**. Action units (AWACS, fighters, etc.) cannot be deleted by players.

### Q: Can I transfer points to teammates?
**A**: If enabled: `-transfer <amount> <player-name>`
If not enabled, this command won't work.

### Q: What's a good starting strategy for earning points?
**A**: 
- Fly CAP and get air kills
- Air to Ground
- Transport troops/cargo

## Troubleshooting

### Q: I'm stuck in spectator and can't slot!
**A**: 
- Did you register? (type `blue` or `red`)
- Are you out of lives? (check `-lives`)
- Is the slot occupied?
- Try different slot

### Q: Commands don't work!
**A**: 
- Check spelling
- Include dash `-` prefix (except `blue`/`red`)
- Verify you have permission
- Some commands require admin

### Q: Game crashed/disconnected, did I lose my life?
**A**: Depends on when:
- Crashed during loading: Usually no
- Crashed in flight: Might lose life if aircraft was destroyed
- Contact admin if unfair death

## Still Have Questions?

Ask in:
- **Discord**: [https://discord.gg/wAsBEfse](https://discord.gg/wAsBEfse)

