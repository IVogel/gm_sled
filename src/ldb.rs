use std::ops::{Deref, DerefMut};
use std::ptr::null;

use lua_shared as lua;
use lua_shared::lua_State;

use crate::check_slice;
use crate::ltree::LTree;

#[derive(Debug, Clone)]
pub struct LDb(pub sled::Db);
impl Deref for LDb {
    type Target = sled::Db;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LDb {
    fn deref_mut(&mut self) -> &mut sled::Db {
        &mut self.0
    }
}

impl LDb {
    pub fn l_open(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let db = sled::open(std::str::from_utf8_unchecked(check_slice!(state, 1)))?;
            let ldb = lua::newuserdata(state, std::mem::size_of::<Self>()).cast::<Self>();
            ldb.write(Self(db));
            Self::metatble(state);
            lua::setmetatable(state, -2);
        }
        Ok(1)
    }

    fn lm_name(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            let name = this.name();
            lua::pushlstring(state, name.as_ptr(), name.len());
            Ok(1)
        }
    }

    fn lm_clear(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            this.clear()?;
            Ok(0)
        }
    }

    fn lm_get(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
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
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            this.insert(check_slice!(state, 2), check_slice!(state, 3))?;
            Ok(0)
        }
    }

    fn lm_remove(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            this.remove(check_slice!(state, 2))?;
            Ok(0)
        }
    }

    fn lm_range(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
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
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
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

    fn lm_tree_names(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            let mut iter = this.0.tree_names().into_iter();
            lua::pushfunction(state, move |state| {
                if let Some(tree_name) = iter.next() {
                    lua::pushlstring(state, tree_name.as_ptr(), tree_name.len());
                    Ok(1)
                } else {
                    Ok(0)
                }
            });
            Ok(1)
        }
    }

    fn lm_open_tree(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            let tree = this.open_tree(std::str::from_utf8_unchecked(check_slice!(state, 2)))?;
            let ltree = lua::newuserdata(state, std::mem::size_of::<LTree>()).cast::<LTree>();
            ltree.write(LTree(tree));
            LTree::metatble(state);
            lua::setmetatable(state, -2);
            Ok(1)
        }
    }

    fn lm_generate_id(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            lua::pushnumber(state, this.generate_id()? as _);
            Ok(1)
        }
    }

    fn __gc(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let _ = lua::Lcheckudata(state, 1, lua::cstr!("csldb"))
                .cast::<Self>()
                .read();
            Ok(0)
        }
    }

    fn metatble(state: lua_State) {
        unsafe {
            if lua::Lnewmetatable(state, lua::cstr!("csldb")) {
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
                lua::pushfunction(state, Self::lm_tree_names);
                lua::setfield(state, -2, lua::cstr!("tree_names"));
                lua::pushfunction(state, Self::lm_open_tree);
                lua::setfield(state, -2, lua::cstr!("open_tree"));
                lua::pushfunction(state, Self::lm_generate_id);
                lua::setfield(state, -2, lua::cstr!("generate_id"));
            }
        }
    }
}
