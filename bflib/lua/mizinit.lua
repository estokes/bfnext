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
local bflib = require("bflib")
bflib.initMiz()
