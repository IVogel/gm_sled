// use std::{collections::HashMap, hash::Hash};

use std::io::Cursor;

use ldb::LDb;
use buffer::Buffer;
use lua_shared as lua;
use lua_shared::lua_State;

mod buffer;
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

#[macro_export]
macro_rules! insert_function {
    ($state:ident, $name:expr, $func:expr) => {
        lua_shared::pushfunction($state, $func);
        lua_shared::setfield($state, -2, lua::cstr!($name));
    };
}

#[no_mangle]
unsafe extern "C" fn gmod13_open(state: lua_State) -> i32 {
    lua::createtable(state, 0, 1);
    insert_function!(state, "Open", LDb::l_open);
    insert_function!(state, "Buffer", Buffer::l_new);
    lua::pushstring(state, lua::cstr!("Sled 0.34.7"));
    lua::setfield(state, -2, lua::cstr!("_VERSION"));
    lua::setfield(state, lua::GLOBALSINDEX, lua::cstr!("sled"));
    match lua::loadx(state, &mut Cursor::new(include_str!("lib.lua")), lua::cstr!("@includes/modules/lsled.lua"), lua::cstr!("t")) {
        Ok(_) => match lua::pcall(state, 0, 0, 0) {
            lua::Status::RuntimeError |
            lua::Status::MemoryError  |
            lua::Status::Error => {lua::error(state);},
            _ => {}
        },
        Err(lua::LError::MemoryError(e)) | 
        Err(lua::LError::SyntaxError(e)) => {
            lua::pushlstring(state, e.as_ptr(), e.as_bytes().len());
            lua::error(state);
        },
        _ => {}
    }
    0
}

#[no_mangle]
unsafe extern "C" fn gmod13_close(_state: lua_State) -> i32 {
    0
}
