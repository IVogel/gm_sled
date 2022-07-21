use std::ops::{Deref, DerefMut};
use std::ptr::null;

use lua_shared as lua;
use lua_shared::lua_State;

use crate::lua_struct::StructError;
use crate::{check_slice, insert_function, lua_struct, tree_get_key, tree_get_no_arg};

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
                Ok(1)
            } else {
                Ok(0)
            }
        }
    }

    fn lm_get_struct(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            let key = check_slice!(state, 2);
            let fmt = check_slice!(state, 3);
            if let Some(ivec) = this.0.get(key)? {
                match lua_struct::unpack(state, fmt, &ivec) {
                    Ok(args) => Ok(args),
                    Err(e) => {
                        drop(ivec);
                        match e {
                            StructError::Error(e) => lua::Lerror(state, e),
                            StructError::ArgError(arg, e) => lua::Largerror(state, arg, e),
                            StructError::InvalidFormatOption(e, opt) => lua::Lerror(state, e, opt),
                            StructError::IOError(e) => return Err(e)?,
                        }
                    }
                }
            } else {
                Ok(0)
            }
        }
    }

    fn lm_insert(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            this.insert(check_slice!(state, 2), check_slice!(state, 3))?;
            Ok(0)
        }
    }

    fn lm_insert_struct(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            let key = check_slice!(state, 2);
            let fmt = check_slice!(state, 3);
            let value = match lua_struct::pack(state, fmt, 4) {
                Ok(result) => result,
                Err(StructError::Error(e)) => lua::Lerror(state, e),
                Err(StructError::ArgError(arg, e)) => lua::Largerror(state, arg, e),
                Err(StructError::InvalidFormatOption(e, opt)) => lua::Lerror(state, e, opt),
                Err(StructError::IOError(e)) => return Err(e)?,
            };
            this.insert(key, value)?;
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

    fn lm_flush(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            lua::pushinteger(state, this.flush()? as _);
            Ok(1)
        }
    }

    fn lm_checksum(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            lua::pushinteger(state, this.checksum()? as _);
            Ok(1)
        }
    }

    fn lm_contains_key(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslt")).cast::<Self>();
            let key = check_slice!(state, 2);
            lua::pushboolean(state, this.contains_key(key)? as _);
            Ok(1)
        }
    }

    tree_get_key!(get_lt get_gt, "cslt");
    tree_get_no_arg!(first last pop_max pop_min, "cslt");

    fn __gc(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let _ = lua::Lcheckudata(state, 1, lua::cstr!("cslt"))
                .cast::<Self>()
                .read();
            Ok(0)
        }
    }

    pub unsafe fn metatable(state: lua_State) {
        if lua::Lnewmetatable(state, lua::cstr!("cslt")) {
            lua::pushvalue(state, -1);
            lua::setfield(state, -2, lua::cstr!("__index"));
            insert_function!(state, "__gc", Self::__gc);
            insert_function!(state, "Name", Self::lm_name);
            insert_function!(state, "Clear", Self::lm_clear);
            insert_function!(state, "Get", Self::lm_get);
            insert_function!(state, "GetStruct", Self::lm_get_struct);
            insert_function!(state, "Insert", Self::lm_insert);
            insert_function!(state, "InsertStruct", Self::lm_insert_struct);
            insert_function!(state, "Remove", Self::lm_remove);
            insert_function!(state, "Range", Self::lm_range);
            insert_function!(state, "ScanPrefix", Self::lm_scan_prefix);
            insert_function!(state, "Flush", Self::lm_flush);
            insert_function!(state, "Checksum", Self::lm_checksum);
            insert_function!(state, "ContainsKey", Self::lm_contains_key);
            insert_function!(state, "GetLT", Self::lm_get_lt);
            insert_function!(state, "GetLTStruct", Self::lm_get_lt_struct);
            insert_function!(state, "GetGT", Self::lm_get_gt);
            insert_function!(state, "GetGTStruct", Self::lm_get_gt_struct);
            insert_function!(state, "First", Self::lm_first);
            insert_function!(state, "FirstStruct", Self::lm_first_struct);
            insert_function!(state, "Last", Self::lm_last);
            insert_function!(state, "LastStruct", Self::lm_last_struct);
            insert_function!(state, "PopMax", Self::lm_pop_max);
            insert_function!(state, "PopMaxStruct", Self::lm_pop_max_struct);
            insert_function!(state, "PopMin", Self::lm_pop_min);
            insert_function!(state, "PopMinStruct", Self::lm_pop_min_struct);
        }
    }
}
