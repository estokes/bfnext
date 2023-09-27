-- set up the dll search path and load the rust library 
package.cpath = package.cpath .. ";" .. lfs.writedir() .. "\\Scripts\\?.dll"
log.write("bflib", log.INFO, "cpath = " .. package.cpath)

log.write("bflib", log.INFO, "loading rust library")
local bflib = require("bflib")

world.addEventHandler(bflib)
