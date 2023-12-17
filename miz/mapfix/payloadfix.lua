

local missionPath = arg[1]
local configPath = arg[2]

print("Mission Path: "..missionPath)
print("Config Path: "..configPath)

--------------


require "BFBase"
local open = io.open
local JSON = require "JSON"

local minizip = require('minizip')

local serializer = require("Serializer")

local zipFile, err = minizip.unzOpen(missionPath, 'rb')
if zipFile == nil then
    print("couldnt find file")
    os.exit()
end

--- LOAD MISSION
local misStr
if zipFile:unzLocateFile("mission") then misStr = zipFile:unzReadAllCurrentFile(false) end

loadstring(misStr)()

if mission then
    print("mission loaded ")
end

-- LOAD DICTIONARY
local dictionaryStr
if zipFile:unzLocateFile("l10n/DEFAULT/dictionary") then dictionaryStr = zipFile:unzReadAllCurrentFile(false) end

loadstring(dictionaryStr)()

zipFile:unzClose()

if dictionary then
    print("dictionary loaded")
end


--- LOAD BFConfig
local function readFile(path)
    local file = open(path, "rb") -- r read mode and b binary mode
    if not file then return nil end
    local content = file:read "*a" -- *a or *all reads the whole file
    file:close()
    return content
end

local function copyFile(src, dst, blocksize)
    blocksize = blocksize or (1024*1024)
    local sf, df, err
    local function bail(...)
        if sf then sf:close() end
        if df then df:close() end
        return ...
    end
    sf, err = io.open(src)
    if not sf then return bail(nil, err) end
    df, err = io.open(dst, "wb")
    if not df then return bail(nil, err) end
    while true do
        local ok, data
        data = sf:read(blocksize)
        if not data then break end
        ok, err = df:write(data)
        if not ok then return bail(nil, err) end
    end
    return bail(true)
end

-- read mission config EDIT THIS FOR SERVER PATH
local fileContent = readFile(configPath);

if fileContent then
    print("Read BF Config")
else
    print("ERROR - Could not read BF Config - " .. configPath)
    os.exit()
end


BFConfig = {}
BFConfig = JSON:decode(fileContent) -- global for other scripts

local _setupPath = "setups." .. BFConfig.era .. "." .. BFConfig.map
require(_setupPath)
-- local _aircraft = readFile(_setupPath)

if _aircraft and type(_aircraft) == "table" and _aircraft["Mi-24P"] then
    print("Read Setup - " .. BFConfig.era .. " " ..  BFConfig.map)
else
    print("ERROR - Could not read Setup - " .. BFConfig.era .. " " ..  BFConfig.map)
    os.exit()
end

BFMiniBase.loadBases(BFConfig.baseConfig)

function generateSTN(previousSTN)
	local s = ""
	local newSTN = previousSTN
	repeat 
		newSTN = newSTN + 1
		s = tostring(newSTN)		
	until string.find(s, "[89]") == nil
	local pad = 5 - string.len(s)
	for i = 1, pad do
		s = "0" .. s
	end
	return s
end

-- FROM MIST
local STN = 0
for coa_name, coa_data in pairs(mission.coalition) do

    if (coa_name == 'red' or coa_name == 'blue') and type(coa_data) == 'table' then

        if coa_data.country then --there is a country table
            
			for cntry_id, cntry_data in pairs(coa_data.country) do

                if type(cntry_data) == 'table' then --just making sure

                    for obj_type_name, obj_type_data in pairs(cntry_data) do

                        if obj_type_name == "helicopter" or obj_type_name == "plane" then --should be an unncessary check

                            if ((type(obj_type_data) == 'table') and obj_type_data.group and (type(obj_type_data.group) == 'table') and (#obj_type_data.group > 0)) then --there's a group!

                                for group_num, group_data in pairs(obj_type_data.group) do

                                    if group_data and group_data.units and type(group_data.units) == 'table' then --making sure again- this is a valid group

                                        for unit_num, unit_data in pairs(group_data.units) do

                                            if unit_data.skill and unit_data.skill:lower() == "client" then
											
												-- print("checking unit - " .. unit_data.name .. " | group - " .. group_data.name)

                                                local _base = BFMiniBase.findBase({ x = unit_data.x, z = unit_data.y, y = unit_data.y })

                                                if _base then
													local unitparams = { 
														unit = unit_data.name,
														group = group_data.name,
														type = unit_data.type,
														coalition = coa_name,
														country_id = cntry_id,
														obj_type = obj_type_name,
														group_num = group_num,
														unit_num = unit_num													
													}
                                                    
													if _aircraft[unit_data.type] then

                                                        local _newLoadout = _aircraft[unit_data.type].default
                                                        local _msg = ""

                                                        if _aircraft[unit_data.type][_base:getBaseType()] then
                                                            _newLoadout = _aircraft[unit_data.type][_base:getBaseType()]
                                                        end

                                                        if _newLoadout.payload then
                                                            unit_data.payload = _newLoadout.payload
                                                            _msg = _msg .. "payload "
                                                        end

                                                        if _newLoadout.livery_id then
                                                            unit_data.livery_id = _newLoadout.livery_id
                                                            _msg = _msg .. "livery "
                                                        end

                                                        if _newLoadout.Radio then
                                                            unit_data.Radio = _newLoadout.Radio
                                                            _msg = _msg .. "frequencies "
                                                        end

                                                        if _newLoadout.AddPropAircraft then

															if type(unit_data.AddPropAircraft) == 'table' and unit_data.AddPropAircraft["STN_L16"] ~= nil then
																unit_data.AddPropAircraft = BFMiniBase.utils.deepCopy(_newLoadout.AddPropAircraft)
																local _stn = generateSTN(STN)
																unit_data.AddPropAircraft["STN_L16"] = _stn
																STN = tonumber(_stn)
																unitparams["STN"] = _stn
															else
																unit_data.AddPropAircraft = _newLoadout.AddPropAircraft
															end
															_msg = _msg .. "properties "
                                                        end
														
														print("Fixed ".. _msg .. " on - "..unit_data.type)
													
													end
                                                    
													_base:addUnit(unitparams)
													
                                                else
                                                    print("NO BASE FOR CLIENT at "..unit_data.name.." for "..unit_data.type)
                                                end
                                            end
                                        end --for unit_num, unit_data in pairs(group_data.units) do
                                    end --if group_data and group_data.units then
                                end --for group_num, group_data in pairs(obj_type_data.group) do
                            end --if ((type(obj_type_data) == 'table') and obj_type_data.group and (type(obj_type_data.group) == 'table') and (#obj_type_data.group > 0)) then
                        end --if obj_type_name == "helicopter" or obj_type_name == "ship" or obj_type_name == "plane" or obj_type_name == "vehicle" or obj_type_name == "static" then
                    end --for obj_type_name, obj_type_data in pairs(cntry_data) do
                end --if type(cntry_data) == 'table' then
            end --for cntry_id, cntry_data in pairs(coa_data.country) do
        end --if coa_data.country then --there is a country table
    end --if coa_name == 'red' or coa_name == 'blue' and type(coa_data) == 'table' then
end --for coa_name, coa_data in pairs(mission.coalition) do

BFMiniBase.fixAllUnits()

local f = io.open("mission", 'w')
if f then
    local s = serializer.new(f)
    s:serialize_simple2("mission", mission)
    f:close()
end

local f = io.open("dictionary", 'w')
if f then
    local s = serializer.new(f)
    s:serialize_simple2("dictionary", dictionary)
    f:close()
end


print("Fixed Payloads & Names - Saved dictionary and mission ")



