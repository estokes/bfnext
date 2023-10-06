package.cpath = package.cpath .. ";" .. lfs.writedir() .. "\\Scripts\\?.dll"
local bflib = require("bflib")
bflib.initHooks()

