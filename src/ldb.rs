use std::ops::{Deref, DerefMut};
use std::ptr::null;

use lua_shared as lua;
use lua_shared::lua_State;

use crate::ltree::LTree;
use crate::lua_struct::StructError;
use crate::{check_slice, insert_function, lua_struct, tree_get_key, tree_get_no_arg};

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
            Self::metatable(state);
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
                Ok(1)
            } else {
                Ok(0)
            }
        }
    }

    fn lm_get_struct(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
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
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            this.insert(check_slice!(state, 2), check_slice!(state, 3))?;
            Ok(0)
        }
    }

    fn lm_insert_struct(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
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
            LTree::metatable(state);
            lua::setmetatable(state, -2);
            Ok(1)
        }
    }

    fn lm_drop_tree(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            lua::pushboolean(
                state,
                this.drop_tree(std::str::from_utf8_unchecked(check_slice!(state, 2)))? as _,
            );
            Ok(1)
        }
    }

    fn lm_was_recovered(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            lua::pushboolean(state, this.0.was_recovered() as _);
            Ok(1)
        }
    }

    fn lm_size_on_disk(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            lua::pushinteger(state, this.size_on_disk()? as _);
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

    fn lm_export(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            let export: Vec<(Vec<u8>, Vec<u8>, Vec<Vec<Vec<u8>>>)> = this
                .export()
                .into_iter()
                .map(|(collection, name, kv)| {
                    (
                        collection,
                        name,
                        kv.into_iter().collect::<Vec<Vec<Vec<u8>>>>(),
                    )
                })
                .collect();
            let blob = bincode::serialize(&export)?;
            lua::pushlstring(state, blob.as_ptr(), blob.len());
            Ok(1)
        }
    }

    fn lm_import(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            let blob = check_slice!(state, 2);
            let import: Vec<(Vec<u8>, Vec<u8>, Vec<Vec<Vec<u8>>>)> = bincode::deserialize(blob)?;
            this.import(
                import
                    .into_iter()
                    .map(|(collection, name, kv)| (collection, name, kv.into_iter()))
                    .collect(),
            );
            Ok(0)
        }
    }

    fn lm_flush(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            lua::pushinteger(state, this.flush()? as _);
            Ok(1)
        }
    }

    fn lm_checksum(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            lua::pushinteger(state, this.checksum()? as _);
            Ok(1)
        }
    }

    fn lm_contains_key(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
            let key = check_slice!(state, 2);
            lua::pushboolean(state, this.contains_key(key)? as _);
            Ok(1)
        }
    }

    tree_get_key!(get_lt get_gt, "csldb");
    tree_get_no_arg!(first last pop_max pop_min, "csldb");

    fn __gc(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let _ = lua::Lcheckudata(state, 1, lua::cstr!("csldb"))
                .cast::<Self>()
                .read();
            Ok(0)
        }
    }

    pub unsafe fn metatable(state: lua_State) {
        if lua::Lnewmetatable(state, lua::cstr!("csldb")) {
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
            insert_function!(state, "TreeNames", Self::lm_tree_names);
            insert_function!(state, "OpenTree", Self::lm_open_tree);
            insert_function!(state, "GenerateID", Self::lm_generate_id);
            insert_function!(state, "Export", Self::lm_export);
            insert_function!(state, "Import", Self::lm_import);
            insert_function!(state, "DropTree", Self::lm_drop_tree);
            insert_function!(state, "WasRecovered", Self::lm_was_recovered);
            insert_function!(state, "SizeOnDisk", Self::lm_size_on_disk);
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
