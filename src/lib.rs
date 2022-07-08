// use std::{collections::HashMap, hash::Hash};

use ldb::LDb;
use lua_shared as lua;
use lua_shared::lua_State;

mod ldb;
mod ltree;

#[macro_export]
macro_rules! check_slice {
    ($state:ident, $index:tt) => {{
        let mut len = 0;
        let str_ptr = lua_shared::Lchecklstring($state, $index, &mut len);
        std::slice::from_raw_parts(str_ptr, len)
    }};
}

#[no_mangle]
unsafe extern "C" fn gmod13_open(state: lua_State) -> i32 {
    lua::createtable(state, 0, 1);
    lua::pushfunction(state, LDb::l_open);
    lua::setfield(state, -2, lua::cstr!("open"));
    lua::pushstring(state, lua::cstr!("Sled 0.34.7"));
    lua::setfield(state, -2, lua::cstr!("_VERSION"));
    lua::setfield(state, lua::GLOBALSINDEX, lua::cstr!("sled"));
    0
}

#[no_mangle]
unsafe extern "C" fn gmod13_close(_state: lua_State) -> i32 {
    0
}
