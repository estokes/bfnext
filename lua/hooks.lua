-- set up the dll search path and load the rust library 
package.cpath = package.cpath .. ";" .. lfs.writedir() .. "\\Scripts\?.dll"
log.write("bflib", log.INFO, "cpath = " .. package.cpath)

log.write("bflib", lig.INFO, "loading rust library")
local bflib = require("bflib")

DCS.setUserCallbacks({
      onPlayerTryConnect = bflib.onPlayerTryConnect,
      onPlayerTrySendChat = bflib.onPlayerTrySendChat,
      onPlayerTryChangeSlot = bflib.onPlayerTryChangeSlot
})
