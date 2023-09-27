-- set up the dll search path and load the rust library 
package.cpath = package.cpath .. ";" .. lfs.writedir() .. "\\Scripts\\?.dll"
local bflib = require("bflib")
bflib:initMiz()
