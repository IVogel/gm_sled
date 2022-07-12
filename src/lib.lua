local sled_open = sled.Open
local cache = setmetatable({}, {__mode = "v"})

function sled.Open(name)
    local db = cache[name]
    if db then return db end
    db = sled_open(name)
    cache[name] = db
    return db
end
