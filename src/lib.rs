use buffer::Buffer;
use ldb::LDb;
use lua_shared as lua;
use lua_shared::lua_State;

mod buffer;
mod ldb;
mod ltree;
mod lua_struct;
mod macros;

#[no_mangle]
unsafe extern "C" fn gmod13_open(state: lua_State) -> i32 {
    lua::createtable(state, 0, 1);
    insert_function!(state, "Open", LDb::l_open);
    insert_function!(state, "Buffer", Buffer::l_new);
    lua::pushstring(state, lua::cstr!("Sled 0.34.7"));
    lua::setfield(state, -2, lua::cstr!("_VERSION"));
    lua::setglobal!(state, lua::cstr!("sled"));
    {
        let code = include_str!("lib.lua");
        match lua::Lloadbufferx(
            state,
            code.as_ptr(),
            code.as_bytes().len(),
            lua::cstr!("@includes/modules/lsled.lua"),
            lua::cstr!("t"),
        ) {
            lua::Status::Ok => match lua::pcall(state, 0, 0, 0) {
                lua::Status::RuntimeError | lua::Status::MemoryError | lua::Status::Error => {
                    lua::error(state);
                }
                _ => {}
            },
            _ => {
                lua::error(state);
            }
        }
    }
    0
}

#[no_mangle]
unsafe extern "C" fn gmod13_close(_state: lua_State) -> i32 {
    0
}
