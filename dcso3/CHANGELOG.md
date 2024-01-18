## 0.1.0

- warehouse system
- StaticObject::get_by_name can return an airbase
- ai tasking system converted to enum
- ai command system converted to enum

## 0.0.10

- fix cargo category and keywords
- implement laser spot
- implement object ids for object, unit, and group
- implement a few additional vec3 and vec2 functions

## 0.0.9

- fix get_velocity was calling getPosition
- catch panics in all callbacks
- fix order of args in on_player_try_connect

## 0.0.8

- fix lifetimes on some returned objects were not as long as they should have been

## 0.0.7

- wherever possible transition error handing to anyhow
- some callback type changes
- a lot of bugs fixed
- bind the land module
- bind world.searchVolume and world.removeJunk

## 0.0.6

- add bindings for most of the triggers singleton, including almost all of the f10 map
shape options
- add bindings for the missionCommands singleton

## 0.0.5

- separate the two DCS lua environments in a statically type safe way
- add new events
- convert u32 returns to i64, which is the native lua type internally

## 0.0.4

- bug fixes

## 0.0.3

- add env.mission
- add timer
- harden returns to dcs because it crashes on lua errors
- fix a lot of bugs

## 0.0.2

- add missing UserHooks callbacks

## 0.0.1

initial release
