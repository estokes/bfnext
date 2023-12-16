-- Copyright (C) 2006, Eagle Dynamics.
-- Serialization module based on the sample from the book
-- "Programming in Lua" by Roberto Ierusalimschy. - Rio de Janeiro, 2003

module('Serializer', package.seeall)
mtab = { __index = _M }

local Factory = require('Factory')

function new(fout)
  return Factory.create(_M, fout)
end

function construct(self, fout)
  self.fout = fout
end

function basicSerialize(self, o)
  if type(o) == "number" then
    return o
  elseif type(o) == "boolean" then
    return tostring(o)
  elseif type(o) == "string" then-- assume it is a string
    return string.format("%q", o)
  else
	return "nil"
  end
end

-- Use third argument as a local table for saved table names accumulation
-- to avoid repeated serialization.
-- Данный вариант позволяет сериализовать таблицы с произвольными символьными ключами.

function serialize_failure_message(self,value,name)
	if name ~= nil then
		return "nil,--Cannot serialize a "..type(value).." :"..name.."\n"
	else
		return "nil,--Cannot serialize a "..type(value).."\n"
	end
end

function failed_to_serialize(self,value,name)
	self.fout:write(serialize_failure_message(self,value,name))
end

function serialize(self, name, value, saved)
  saved = saved or {}
  self.fout:write(name, " = ")
  if type(value) == "number" or type(value) == "string" or type(value) == "boolean" then
    self.fout:write(self:basicSerialize(value), "\n")
  elseif type(value) == "table" then
    if saved[value] then -- value already saved?
      self.fout:write(saved[value], "\n") -- use its previous name
    else
      saved[value] = name -- save name for next time
      self.fout:write("{}\n") -- create a new table
      for k,v in pairs(value) do -- serialize its fields
        local fieldname = string.format("%s[%s]", name, self:basicSerialize(k))
        self:serialize(fieldname, v, saved)
      end
    end
  else
	self:failed_to_serialize(value,name)
  end
end

-- Более наглядная и простая сериализация без экономии повторяющихся таблиц.
-- Предполагается, что символьные ключи в таблицах являются идентификаторами Lua.
function serialize_simple(self, name, value, level)
  if level == nil then level = "" end
  if level ~= "" then level = level.."  " end
  self.fout:write(level, name, " = ")
  if type(value) == "number" or
	 type(value) == "string" or 
	 type(value) == "boolean" then 
    self.fout:write(self:basicSerialize(value), ",\n")
  elseif type(value) == "table" then
      self.fout:write("\n"..level.."{\n") -- create a new table
      for k,v in pairs(value) do -- serialize its fields
        local key
        if type(k) == "number" then
          key = string.format("[%s]", k)
        else
          key = k
        end
        self:serialize_simple(key, v, level.."  ")
      end
      if level == "" then
        self.fout:write(level.."} -- end of "..name.."\n")
      else
        self.fout:write(level.."}, -- end of "..name.."\n")
      end
  else
	self:failed_to_serialize(value,name)
  end
end

-- Более наглядная и простая сериализация без экономии повторяющихся таблиц.
-- Предполагается, что символьные ключи в таблицах не являются идентификаторами Lua,
-- но не содержат апострофов.
function serialize_simple2(self, name, value, level)
  if level == nil then level = "" end
  if level ~= "" then level = level.."  " end
  self.fout:write(level, name, " = ")
  if type(value) == "number" or type(value) == "string" or type(value) == "boolean" then
    self.fout:write(self:basicSerialize(value), ",\n")
  elseif type(value) == "table" then
      self.fout:write("\n"..level.."{\n") -- create a new table
      for k,v in pairs(value) do -- serialize its fields
        local key
        if type(k) == "number" then
          key = string.format("[%s]", k)
        else
          key = string.format("[%q]", k)
        end
        self:serialize_simple2(key, v, level.."  ")
      end
      if level == "" then
        self.fout:write(level.."} -- end of "..name.."\n")
      else
        self.fout:write(level.."}, -- end of "..name.."\n")
      end
  else
	self:failed_to_serialize(value,name)
  end
end

-- serialization to string

local serialize_to_string_result

function add_to_string(str)
  serialize_to_string_result = serialize_to_string_result..str    
end --

function serialize_to_string(self, name, value)
  serialize_to_string_result = ""
  self:serialize_to_string_simple(name, value)
  return serialize_to_string_result
end -- func                              

function serialize_to_string_simple(self, name, value,level)
    local level   =  level or ""
    add_to_string(level..name.."=")
    if  type(value) == "number" or 
        type(value) == "string" or 
        type(value) == "boolean" then
        add_to_string(self:basicSerialize(value) .. ",\n")
    elseif type(value) == "table" then
        add_to_string("\n"..level.."{\n")
		
		local ipaired = {
		}
		for k,v in ipairs(value) do -- serialize its fields
			ipaired[k] = true
            self:serialize_to_string_simple(string.format("[%s]", k),v,level.."\t")
        end
        for k,v in pairs(value) do -- serialize its fields
			if ipaired[k] == nil then
				local key
				if type(k) == "number" then          key = string.format("[%s]"  , k)
				else                                      key = string.format("[%q]", k)         end
				self:serialize_to_string_simple(key,v,level.."\t")
			end
        end
        if level == "" then   add_to_string(level.."}\n")
        else                  add_to_string(level.."},\n") end
    else   
		add_to_string(self:serialize_failure_message(value,name))
    end
end -- func

function serialize_to_string_noCR(self, name, value)
  serialize_to_string_result = ""
  self:serialize_to_string_simple_noCR(name, value)
  -- delete last ","
  return string.sub(serialize_to_string_result,1,string.len(serialize_to_string_result)-1)
end -- func                              

function serialize_to_string_simple_noCR(self, name, value)
  add_to_string(name.."=")
  if type(value) == "number" or type(value) == "string" or type(value) == "boolean" then
      add_to_string(self:basicSerialize(value) .. ",")
  elseif type(value) == "table" then
      add_to_string("{")
      for k,v in pairs(value) do -- serialize its fields
        local key
        if type(k) == "number" then
          key = string.format("[%s]", k)
        else
          key = string.format("['%s']", k)
        end
        self:serialize_to_string_simple_noCR(key, v)
      end
      add_to_string("},")
  else
      add_to_string(self:serialize_failure_message(value,name))
  end
end -- func

local function isSimpleTable(value)
    if value == nil or type(value) ~= "table" then
        return true
    end

    for k,v in pairs(value) do
        if type(v) == "table" or type(k) ~= "number" then return false end
    end
    return true
end

-- Аналогична serialize_simple но простые табилцы (не содержащие влаженных таблиц), записываются в одну строку в тщетной надежде повысить читаемость.

function serialize_compact_iter(self, name, value, level,iterator_fn)
  local endOfLineSymb 
  if level == nil then 
	level = "" 
	endOfLineSymb = "\n"
  else
	endOfLineSymb = ",\n"
  end
  
  local v_type = type(value) 

  if v_type == "number"  or 
	 v_type == "string"  or 
	 v_type == "boolean" then
	self.fout:write(level, name, "\t=\t") 
	self.fout:write(self:basicSerialize(value), endOfLineSymb)
  elseif v_type == "table" then
	  self.fout:write(level, name, " = ") 
      if not isSimpleTable(value) then
          self.fout:write("\n"..level.."{\n") -- create a new table
          for k,v in iterator_fn(value) do -- serialize its fields
            local key
            if type(k) == "number" then      key = string.format("[%s]", k)
            elseif type(k) == "string" then
				local match_result = string.match(k,'[_%a][_%w]*')--match that is valid lua identifier
				if match_result and match_result == k then
					key = k
				else
					key = string.format("['%s']", k)
				end
            else
				key = k
            end
            serialize_compact_iter(self, key, v, level..'\t',iterator_fn)
          end
		  
          if level == "" then	self.fout:write(level.."} -- end of "..name.."\n")
          else		            self.fout:write(level.."}, -- end of "..name.."\n")
          end
      else
          self.fout:write("\t{") -- create a new table
          for i,v in ipairs(value) do -- serialize its fields
            if (i == #value) then self.fout:write(self:basicSerialize(v)) 
            else self.fout:write(self:basicSerialize(v), ",\t")
            end
          end
          self.fout:write("}"..endOfLineSymb)
      end
  else
	self.fout:write(level, name, "\t=\t") 
	self:failed_to_serialize(value,name)
  end
end

function serialize_compact(self, name, value, level)
	serialize_compact_iter(self, name, value, level,pairs)
end

-- превращает таблицу в массив отсортированных по ключу пар [ключ, значение]
function getSortedPairs(tableValue)
  local result = {}
  
  for key, value in pairs(tableValue) do
    table.insert(result, {key = key, value = value})
  end
  
  local sortFunction = function (pair1, pair2) 
    return pair1.key < pair2.key 
  end
  
  table.sort(result, sortFunction)
  
  return result
end

-- сохраняет в файл таблицу, отсортированную по ключу 
-- это нужно для удобства сравнения сохраненных таблиц svn'ом, 
function serialize_sorted(self, name, value, level)
  local levelOffset = "\t"
  
  if level == nil then 
    level = "" 
  end
  
  -- if level ~= "" then 
    -- level = level .. levelOffset 
  -- end
  
  self.fout:write(level, name, " = ")
  
  local valueType = type(value)
  
  if valueType == "number" or 
     valueType == "string" or 
	 valueType == "boolean" then
    self.fout:write(self:basicSerialize(value), ",\n")
  elseif valueType == "table" then
      self.fout:write("{\n") -- create a new table
      
      local sortedPairs = getSortedPairs(value)
      
      for i, pair in pairs(sortedPairs) do
        local k = pair.key        
        local key
        
        if type(k) == "number" then
          key = string.format("[%s]", k)
        else
          key = string.format("[%q]", k)
        end
        
        self:serialize_sorted(key, pair.value, level .. levelOffset)    
      end

      if level == "" then
        self.fout:write(level.."}\n")
      else
        self.fout:write(level.."},\n")
      end
  else
     self:failed_to_serialize(value,name)
  end
end