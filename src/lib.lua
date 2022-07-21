-- TODO: implement basic table-like indexing functionality

local CSLDB_META, CSLT_META = ...;

do
    local sled_open = sled.Open
    local cache = setmetatable({}, {__mode = "v"})

    function sled.Open(name)
        local db = cache[name]
        if db then return db end
        db = sled_open(name)
        cache[name] = db
        return db
    end
end

do
    local tree_cache = setmetatable({}, {__mode = "k"})
    local csldb_open_tree = CSLDB_META.OpenTree

    function CSLDB_META:OpenTree(name)
        local trees = tree_cache[self];
        if not trees then
            trees = setmetatable({}, {__mode = "v"})
            tree_cache[self] = trees
        end
        local tree = trees[name]
        if not tree then
            tree = csldb_open_tree(self, name)
            trees[name] = tree
        end
        return tree
    end
end
