use std::ops::{Deref, DerefMut};
use std::ptr::null;

use lua_shared as lua;
use lua_shared::lua_State;

use crate::check_slice;

#[derive(Debug, Clone)]
pub struct LTree(pub sled::Tree);
impl Deref for LTree {
    type Target = sled::Tree;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LTree {
    fn deref_mut(&mut self) -> &mut sled::Tree {
        &mut self.0
    }
}

impl LTree {
    fn lm_name(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            let name = this.name();
            lua::pushlstring(state, name.as_ptr(), name.len());
            Ok(1)
        }
    }

    fn lm_clear(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            this.clear()?;
            Ok(0)
        }
    }

    fn lm_get(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            if let Some(ivec) = this.0.get(check_slice!(state, 2))? {
                lua::pushlstring(state, ivec.as_ptr(), ivec.len());
            } else {
                lua::pushnil(state)
            }
            Ok(1)
        }
    }

    fn lm_insert(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            this.insert(check_slice!(state, 2), check_slice!(state, 3))?;
            Ok(0)
        }
    }

    fn lm_remove(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            this.remove(check_slice!(state, 2))?;
            Ok(0)
        }
    }

    fn lm_range(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            let mut range = this.range(check_slice!(state, 2)..=check_slice!(state, 3));
            lua::pushfunction(state, move |state| {
                if let Some(tree_name) = range.next() {
                    let (key, value) = tree_name?;
                    lua::pushlstring(state, key.as_ptr(), key.len());
                    lua::pushlstring(state, value.as_ptr(), value.len());
                    Ok(2)
                } else {
                    Ok(0)
                }
            });
            Ok(1)
        }
    }

    fn lm_scan_prefix(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            let prefix = {
                let mut len = 0;
                std::slice::from_raw_parts(lua::Loptlstring(state, 2, null(), &mut len), len)
            };
            let mut prefix = this.scan_prefix(prefix);
            lua::pushfunction(state, move |state| {
                if let Some(tree_name) = prefix.next() {
                    let (key, value) = tree_name?;
                    lua::pushlstring(state, key.as_ptr(), key.len());
                    lua::pushlstring(state, value.as_ptr(), value.len());
                    Ok(2)
                } else {
                    Ok(0)
                }
            });
            Ok(1)
        }
    }

    fn __gc(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let _ = lua::Lcheckudata(state, 1, lua::cstr!("cslt"))
                .cast::<Self>()
                .read();
            Ok(0)
        }
    }

    pub fn metatble(state: lua_State) {
        unsafe {
            if lua::Lnewmetatable(state, lua::cstr!("cslt")) {
                lua::pushvalue(state, -1);
                lua::setfield(state, -2, lua::cstr!("__index"));
                lua::pushfunction(state, Self::__gc);
                lua::setfield(state, -2, lua::cstr!("__gc"));
                lua::pushfunction(state, Self::lm_name);
                lua::setfield(state, -2, lua::cstr!("name"));
                lua::pushfunction(state, Self::lm_clear);
                lua::setfield(state, -2, lua::cstr!("clear"));
                lua::pushfunction(state, Self::lm_get);
                lua::setfield(state, -2, lua::cstr!("get"));
                lua::pushfunction(state, Self::lm_insert);
                lua::setfield(state, -2, lua::cstr!("insert"));
                lua::pushfunction(state, Self::lm_remove);
                lua::setfield(state, -2, lua::cstr!("remove"));
                lua::pushfunction(state, Self::lm_range);
                lua::setfield(state, -2, lua::cstr!("range"));
                lua::pushfunction(state, Self::lm_scan_prefix);
                lua::setfield(state, -2, lua::cstr!("scan_prefix"));
            }
        }
    }
}
