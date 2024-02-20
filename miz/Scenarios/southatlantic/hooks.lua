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

net.log("loading bflib.dll")

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

local function copy_file(from, to)
    local fromf, fe = io.open(from, "rb")
    local tof, te = io.open(to, "wb")
    if fromf == nil then
        return fe
    end
    if tof == nil then
        return te
    end
    tof:write(fromf:read("*a"))
    fromf:close()
    tof:close()
end

local bflib_update_file = lfs.writedir() .. "\\Scripts\\_bflib.dll"
local bflib_dll = lfs.writedir() .. "\\Scripts\\bflib.dll"

if file_exists(bflib_update_file) then
    local e = copy_file(bflib_update_file, bflib_dll)
    if e ~= nil then
       net.log("could not install updated dll " .. e) 
    else
       os.remove(bflib_update_file)
       net.log("installed updated bflib.dll")
    end
end

local bflib = require("bflib")
bflib.initHooks()
