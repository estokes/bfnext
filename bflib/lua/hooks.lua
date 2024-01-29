-- Copyright 2024 Eric Stokes.

-- This file is part of bflib.

-- bflib is free software: you can redistribute it and/or modify it under
-- the terms of the GNU Affero Public License as published by the Free
-- Software Foundation, either version 3 of the License, or (at your
-- option) any later version.

-- bflib is distributed in the hope that it will be useful, but WITHOUT
-- ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
-- FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero Public License
-- for more details.

package.cpath = package.cpath .. ";" .. lfs.writedir() .. "\\Scripts\\?.dll"

local function file_exists(name)
    local f=io.open(name, "r")
    if f ~= nil then
        io.close(f)
        return true
    else
        return false
    end
end

local bflib_update_file = lfs.writedir() .. "\\Scripts\\_bflib.dll"
local bflib_dll = lfs.writedir() .. "\\Scripts\\bflib.dll"

if file_exists(bflib_update_file) then
    local r, e = os.rename(bflib_update_file, bflib_dll)
    if r == nil then
       net.log("could not install updated dll " .. e) 
    else
       net.log("installed updated bflib.dll")
    end
end

local bflib = require("bflib")
bflib.initHooks()
