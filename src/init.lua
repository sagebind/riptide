lish = {
    status = 0,
    directory_stack = {}
}

stdin = nil
stdout = nil
stderr = nil


function lish.prompt()
    if lish.status == 0 then
        return pwd() .. "$ "
    else
        return pwd() .. "|" .. lish.status .. "$ "
    end
end

function read()
    local status, value = coroutine.resume(producer)
    return value
end

function send(...)
    local received = {coroutine.yield(...)}
end


function echo(...)
    local str = table.concat({...}, " ")
    send(str)
end

function print(x)
    send(x)
end

function source(file)
    return dofile(file)
end

function eval(...)
    local str = table.concat({...}, " ")
    local chunk = loadstring(str)
    return chunk()
end


local mt = getmetatable(_G)
if mt == nil then
  mt = {}
  setmetatable(_G, mt)
end

-- set hook for undefined variables
mt.__index = function(t, cmd)
	return function(...)
        return command(cmd, ...)
    end
end

function seq(startIndex, endIndex)
    for i = startIndex, endIndex do
        send(i)
    end
end


dofile("config.lua")


local pipe_mt = {
    -- pipe op
    __bor = function(x)
    end,

    -- & op
    __band = function(x)
    end
}



function pushd(dir)
    table.insert(lish.directory_stack, pwd())
    cd(dir)
end

function popd()
    cd(table.remove(lish.directory_stack))
end
