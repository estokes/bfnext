module('Factory', package.seeall)

function setBaseClass(class, baseClass)
	assert(baseClass.mtab)
	setmetatable(class, baseClass.mtab)
end

function create(class, ...)
	local w = {}

	setBaseClass(w, class)
	w:construct(unpack(arg))

	return w
end

function createClass(class, baseModule)
	if baseModule then
		setmetatable(class, {__index = baseModule})
	end
	
	class.new = function(...)
		local newObject = {}
		setmetatable(newObject, {__index = class})
		
		newObject:construct(...)
		
		return newObject		
	end
	
	return class
end
