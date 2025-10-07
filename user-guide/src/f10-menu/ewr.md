# Early Warning Radar (EWR)

Get real-time radar reports on enemy and friendly aircraft positions for enhanced situational awareness.

## Overview

The EWR system provides:
- Enemy aircraft positions
- Altitude and heading information
- Distance and bearing from your position
- Friendly aircraft locations
- Tactical intelligence

## EWR Menu

Access via F10 → EWR

**Menu Options**:
- **Report**: Enemy aircraft report
- **Toggle**: Enable/disable EWR reports
- **Friendly Report**: Show friendly aircraft
- **Units to Imperial**: Switch to feet/nautical miles
- **Units to Metric**: Switch to meters/kilometers

## Enemy Report

**Request Report**:
```
F10 → EWR → Report
```

**Report Format**:
```
BRAA: Bearing, Range, Altitude, Aspect

BRAA 045/25/15000/HOT
BRAA 180/40/5000/FLANK
BRAA 270/15/20000/COLD
```

**Reading**:
- **Bearing**: Direction from you (degrees)
- **Range**: Distance (nm or km)
- **Altitude**: Height (feet or meters)
- **Aspect**: 
  - HOT: Coming toward you
  - COLD: Going away from you
  - FLANK: Crossing left/right
  - BEAM: 90° to you

**Example**:
```
BRAA 045/25/15000/HOT
```
= Enemy at 045°, 25nm away, 15,000ft, coming toward you

## Friendly Report

**Request Report**:
```
F10 → EWR → Friendly Report
```

Shows same format for friendly aircraft:
- Your team's aircraft positions
- Coordination information

## Toggle EWR

**Enable/Disable**:
```
F10 → EWR → Toggle
```

**Effect**:
- ON: Automatic periodic reports
- OFF: Manual request only
- Preference per player

**Use Cases**:
- ON: High-threat environment
- OFF: Reduce message spam

## Unit Systems

**Imperial**:
```
F10 → EWR → Units to Imperial
```
- Feet (altitude)
- Nautical miles (range)
- Standard for aviation

**Metric**:
```
F10 → EWR → Units to Metric
```
- Meters (altitude)
- Kilometers (range)
- International standard

**Changes apply immediately**

## EWR Limitations

**Not Real-Time**:
- Report is snapshot at time of request
- Aircraft move constantly between reports

**Coverage Limits**:
- Depends on deployed EWR radar sites
- Terrain affects radar coverage
- Low-altitude contacts may not appear

**No Identification**:
- Doesn't specify aircraft type
- All contacts shown generically

## See Also

- [Actions Menu](./actions.md) - Deploy AWACS for better coverage

