# gm_sled

gm_sled is really simple wrapper around [sled](https://github.com/spacejam/sled) key-value database.

# Installation
Download latest binary for corresponding OS and throw it into `garrysmod/lua/bin/`.  
Or, you can compile your own version.

Compilation for main Garry's Mod branch:
```bash
cargo build --release --target i686-unknown-linux-gnu
```
Compilation for x86-x64 branch:
```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

# Example usage (but not really good example)
```lua
require("sled")

local db = sled.Open("currencydb")

currencydb = {__currency = {}}

function currencydb.set(player, currency, value)
    local tree = currencydb.__currency[currency]
    if not tree then
        tree = db:OpenTree(currency)
        currencydb.__currency[currency] = tree
    end
    tree:InsertStruct(player:SteamID64(), "d", value)
end

function currencydb.get(player, currency)
    local tree = currencydb.__currency[currency]
    if not tree then
        tree = db:OpenTree(currency)
        currencydb.__currency[currency] = tree
    end
    return tree:GetStruct(player:SteamID64(), "d")
end

function currencydb.list()
    local names = {}
    for name in db:TreeNames() do
        if name == "__sled__default" then continue end
        table.insert(names, name)
    end
    return names
end
```