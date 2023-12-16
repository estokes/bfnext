dofile("mission")
dofile("dictionary")

if mission then
    print("mission")
end


function readAll(file)
    local f = assert(io.open(file, "rb"))
    local content = f:read("*all")
    f:close()
    return content
end

-- recursively search

local _groupKeys = {}
local _groupIDs = {}

local _unitKeys = {}
local _unitIDs = {}

local _aircraft = {

    ["Su-27"] = {
        ["type"] = "Su-27",
        ["payload"] =
        {
            ["pylons"] =
            {
            }, -- end of ["pylons"]
            ["fuel"] = 1272,
            ["flare"] = 0,
            ["chaff"] = 0,
            ["gun"] = 100,
        }, -- end of ["payload"]
        ["livery_id"] = "1018 - United Arab Emirates",
    }
}

for coa_name, coa_data in pairs(mission.coalition) do

    if (coa_name == 'red' or coa_name == 'blue') and type(coa_data) == 'table' then

        if coa_data.country then --there is a country table
            for cntry_id, cntry_data in pairs(coa_data.country) do


                if type(cntry_data) == 'table' then	--just making sure

                    for obj_type_name, obj_type_data in pairs(cntry_data) do

                        if obj_type_name == "helicopter" or obj_type_name == "ship" or obj_type_name == "plane" or obj_type_name == "vehicle" or obj_type_name == "static" then --should be an unncessary check

                            local category = obj_type_name

                            if ((type(obj_type_data) == 'table') and obj_type_data.group and (type(obj_type_data.group) == 'table') and (#obj_type_data.group > 0)) then	--there's a group!

                                for group_num, group_data in pairs(obj_type_data.group) do

                                    if group_data and group_data.units and type(group_data.units) == 'table' then	--making sure again- this is a valid group

                                        if not _groupKeys[group_data.name] then

                                            _groupKeys[group_data.name] = dictionary[group_data.name]
                                        elseif _groupKeys[group_data.name] then

                                            print("Duplicate Group Key!" .. group_data.name)
                                            return

                                        end

                                        if not _groupIDs[group_data.groupId] then
                                            _groupIDs[group_data.groupId] = dictionary[group_data.name]
                                        elseif _groupIDs[group_data.groupId] then
                                            print("Duplicate Group ID!" .. group_data.groupId)
                                            return
                                        end



                                        for unit_num, unit_data in pairs(group_data.units) do


                                            if not _unitKeys[unit_data.name] then

                                                _unitKeys[unit_data.name] = dictionary[unit_data.name]
                                            elseif _unitKeys[unit_data.name] then

                                                print("Duplicate Group Key!" .. unit_data.name)
                                                return

                                            end

                                            if not _unitIDs[unit_data.unitId] then
                                                _unitIDs[unit_data.unitId] = dictionary[unit_data.name]
                                            elseif _unitIDs[unit_data.unitId] then
                                                print("Duplicate unitId ID!" .. unit_data.unitId)
                                                return
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

--local mizString = readAll("mission")

--local _groupNames = {}
--local _unitNames = {}
--for _name, _groupName in pairs(dictionary) do
--
--    if string.find(mizString,"\"".._name.."\"") then
--        if string.find(_name,"GroupName") and not _groupNames[_groupName] then
--            _groupNames[_groupName] = _name
--        elseif string.find(_name,"GroupName") and _groupNames[_groupName] then
--            print(_groupName.." DUPLICATE" .." GROUP KEY ".._name.." MATCHES ".._groupNames[_groupName])
--            return
--        end
--
--        if string.find(_name,"UnitName") and not _unitNames[_groupName] then
--            _unitNames[_groupName] = _name
--        elseif string.find(_name,"UnitName") and _unitNames[_groupName] then
--            print(_groupName.." DUPLICATE" .." UNIT KEY ".._name.." MATCHES ".._unitNames[_groupName])
--            return
--        end
--
--    end
--
--
--end

for _key,_name in pairs(_groupIDs) do
    print(_key .." ".._name)
end


print("done")

