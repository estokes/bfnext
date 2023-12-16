--
-- BF MINI BASE - used for processing the BFCONFIG file and naming all the slots

local class = require 'middleclass'

local BFMiniBases = {}
BFMiniBase = class('BFMiniBase')

local squadronTags = { "=BS=", "SF SQN", "64th", "GTAG" }

function BFMiniBase.loadBases(bases)

    for _, _baseConfig in pairs(bases) do

        local base = BFMiniBase:new(_baseConfig)

        if BFMiniBases[base:getId()] then
            print("DUPLICATE BASE "..base:getId())
        else
            BFMiniBases[base:getId()] = base
          --  print("INIT "..base:getName())
        end
    end
end

function BFMiniBase.getBase(id)

    return BFMiniBases[id]
end

function BFMiniBase.getAllBases()

    return BFMiniBases
end

function BFMiniBase.findBase(_point)

    for _,_base in pairs(BFMiniBases) do
        if _base:inBase(_point) then
            return _base
        end

    end

    return nil

end

function BFMiniBase:initialize(baseConfig)

    self.id = baseConfig.id
    self.name = baseConfig.name
    self.baseType = string.upper(baseConfig.baseType)
    self.pak = baseConfig.pak
    self.units = {}

    self.counter = 0

    --init location
    self.location = {}

    for _,_zoneName in pairs(baseConfig.location)do
        local _zone = self:checkZone(_zoneName)
        if _zone then

            -- set offset for height
            _zone.point = BFMiniBase.utils.makeVec3(_zone)

            table.insert(self.location, _zone)
        end
    end

    -- calculate centroid

    local _tx, _ty = 0, 0
    for _index, _zone in ipairs(self.location) do
        _tx = _tx + _zone.point.x
        _ty = _ty + _zone.point.z
    end

    local _npoints = #self.location

    local _point = { x = _tx / _npoints, z = _ty / _npoints }

    _point.y =0
    --save centroid
    self.centroid = _point

end

function BFMiniBase:addUnit(_unit)
    table.insert(self.units,_unit)
end

function BFMiniBase.fixAllUnits()
    for _,_base in pairs(BFMiniBases) do
        _base:fixUnits()
    end

end

function BFMiniBase:fixUnits()

	print("Fixing Units in - " .. self:getName())
    local _sort = function( a,b )

        local _aType = a.type:gsub('%W',''):lower()
        local _bType = b.type:gsub('%W',''):lower()

        return _aType < _bType

    end
    table.sort(self.units,_sort)

    for _,_unit in ipairs(self.units) do
		local _squadronTag = nil
        for _, _tag in pairs(squadronTags) do
            if string.find(_unit.group, _tag, 1, true) ~= nil then
                _squadronTag = _tag
            end
        end

        local _groupName
        if _squadronTag ~= nil and _squadronTag ~= "" then
            _groupName = string.format("%s #%02d - PAK %d - %s",self:getName(), self:getCounterAndIncrement(), self:getPak(), _squadronTag)
            -- dictionary[_unit.group] = string.format("%s #%02d - PAK %d - %s",self:getName(), self:getCounterAndIncrement(), self:getPak(), _squadronTag)
            -- print("found squadron group slot: " .. _groupName)
        else
            _groupName = string.format("%s #%02d - PAK %d",self:getName(), self:getCounterAndIncrement(), self:getPak())
            -- dictionary[_unit.group] = string.format("%s #%02d - PAK %d",self:getName(), self:getCounterAndIncrement(), self:getPak())
            -- print("found regular group slot: " .. _groupName)
        end
		
		local stn = ""
		if _unit["STN"] ~= nil then
			stn = " STN: " .. _unit["STN"]
		end
		
        mission["coalition"][_unit.coalition]["country"][_unit.country_id][_unit.obj_type]["group"][_unit.group_num]["name"] = _groupName .. stn
        print("updated group slot: " .._groupName .. "STN:".. stn)
        mission["coalition"][_unit.coalition]["country"][_unit.country_id][_unit.obj_type]["group"][_unit.group_num]["units"][_unit.unit_num]["name"] =  "Pilot ".._groupName.." - #".._unit.unit_num .. stn
        print("updated unit slot: " .. "Pilot ".._groupName.." - #".._unit.unit_num)
       -- print(dictionary[_unit.group] .." ".._unit.type )
    end

end

function BFMiniBase:fixUnit(unitName, groupName)
	local newUnitName
	local newGroupName
	local _squadronTag = nil
	for _, _tag in pairs(squadronTags) do
		if string.find(groupName, _tag, 1, true) ~= nil then
			_squadronTag = _tag
		end
	end

	if _squadronTag ~= nil and _squadronTag ~= "" then
		newGroupName = string.format("%s #%02d - PAK %d - %s",self:getName(), self:getCounterAndIncrement(), self:getPak(), _squadronTag)
		print("found squadron slot: " .. newGroupName)
	else
		newGroupName = string.format("%s #%02d - PAK %d",self:getName(), self:getCounterAndIncrement(), self:getPak())
	end

	newUnitName =  "Pilot "..newGroupName
	return newUnitName, newGroupName
end

function BFMiniBase:checkZone(_zone)

    for _,_foundZone in pairs(mission.triggers.zones) do
        if  type(_foundZone) == 'table'  and _foundZone.name == _zone then
          --  print("Verified ZONE - " .. self.id .. " : " .. _zone)
            return BFMiniBase.utils.deepCopy(_foundZone)
        end
    end

    print("MISSING ZONE - " .. self.id .. " : " .. _zone)
    return false

end

function BFMiniBase:getId()

    return self.id
end


function BFMiniBase:getName()

    return self.name
end


function BFMiniBase:getBaseType()

    return self.baseType
end

function BFMiniBase:getPak()

    return self.pak
end

function BFMiniBase:increment()

     self.counter = self.counter +1
end

function BFMiniBase:getCounterAndIncrement()

    self:increment()
    return self.counter
end


function BFMiniBase:inBase(_point)

    for _,_zone in pairs(self.location)do

        local dist = BFMiniBase.utils.get2DDist(_point,_zone.point)

        if dist < _zone.radius then
            return true
        end
    end

    return false
end

function BFMiniBase:getDistance(_point)

    return BFMiniBase.utils.get2DDist(_point,self.centroid)
end

function BFMiniBase:getCentroid()

    return self.centroid
end

BFMiniBase.utils = {}

function BFMiniBase.utils.countTable(T)

    if T == nil then
        return 0
    end


    local count = 0
    for _ in pairs(T) do count = count + 1 end
    return count

end

function BFMiniBase.utils.makeVec3(vec, offset)
    local adj = offset or 0

    if not vec.z then
        return {x = vec.x, y = (0), z = vec.y}
    else
        return {x = vec.x, y = (0), z = vec.z}
    end
end

--- Vector magnitude
-- @tparam Vec3 vec vector
-- @treturn number magnitude of vector vec
function BFMiniBase.utils.mag(vec)
    return (vec.x^2 + vec.y^2 + vec.z^2)^0.5
end

function BFMiniBase.utils.get2DDist(point1, point2)
    point1 = BFMiniBase.utils.makeVec3(point1)
    point2 = BFMiniBase.utils.makeVec3(point2)
    return BFMiniBase.utils.mag({x = point1.x - point2.x, y = 0, z = point1.z - point2.z})
end

function BFMiniBase.utils.deepCopy(object)
    local lookup_table = {}
    local function _copy(object)
        if type(object) ~= "table" then
            return object
        elseif lookup_table[object] then
            return lookup_table[object]
        end
        local new_table = {}
        lookup_table[object] = new_table
        for index, value in pairs(object) do
            new_table[_copy(index)] = _copy(value)
        end
        return setmetatable(new_table, getmetatable(object))
    end
    return _copy(object)
end

