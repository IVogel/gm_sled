use lua_shared as lua;
use lua_shared::lua_State;
use paste::paste;
use std::u64;

use crate::{check_slice, insert_function};
macro_rules! def_rw {
    ($type_name:ty) => {
        paste! {
            fn [<read_ $type_name>](&mut self) -> Option<$type_name> {
                if self.0.len() == 0 || self.1 + std::mem::size_of::<$type_name>() > self.0.len() {
                    return None
                }
                let data = self.read(std::mem::size_of::<$type_name>())?;
                unsafe {Some($type_name::from_le(*std::mem::transmute::<_, &$type_name>(data.as_ptr())))}
            }
            fn [<write_ $type_name>](&mut self, value: $type_name) {
                self.write(&value.to_le_bytes());
            }
        }
    };
    ($($type_name:ty )+) => {
        $(def_rw!($type_name);)+
    };
}

macro_rules! def_lmint {
    ($name:ident, $type_name:ty) => {
        paste! {
            fn [<lm_read_ $name>](state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
                unsafe {
                    let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
                    match this.[<read_ $type_name>]() {
                        Some(val) => {
                            lua::pushinteger(state, val as _);
                        }
                        None => {
                            lua::pushnil(state)
                        }
                    }
                    Ok(1)
                }
            }
            fn [<lm_write_ $name>](state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
                unsafe {
                    let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
                    this.[<write_ $type_name>](lua::Lcheckinteger(state, 2) as _);
                    Ok(0)
                }
            }
        }
    };
}

macro_rules! def_lmfloat {
    ($name:ident, $type_name:ty) => {
        paste! {
            fn [<lm_read_ $name>](state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
                unsafe {
                    let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
                    match this.[<read_ $type_name>]() {
                        Some(val) => {
                            lua::pushnumber(state, val as _);
                        }
                        None => {
                            lua::pushnil(state)
                        }
                    }
                    Ok(1)
                }
            }
            fn [<lm_write_ $name>](state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
                unsafe {
                    let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
                    this.[<write_ $type_name>](lua::Lchecknumber(state, 2) as _);
                    Ok(0)
                }
            }
        }
    };
}

pub struct Buffer(Vec<u8>, usize);

impl Buffer {
    fn new(size: usize) -> Self {
        Self(Vec::with_capacity(size), 0)
    }

    fn read(&mut self, bytes: usize) -> Option<&[u8]> {
        if bytes == 0 || self.0.len() == 0 {
            return None;
        }
        let pos = self.1.min(self.0.len());
        let bytes_to_read = if bytes + pos > self.0.len() {
            self.0.len() - pos
        } else {
            bytes
        };
        if bytes_to_read == 0 {
            return None;
        }
        self.1 += bytes_to_read;
        Some(&self.0[pos..pos + bytes_to_read])
    }

    // Straightforward ripoff from Cursor.
    fn write(&mut self, data: &[u8]) {
        if self.1 > self.0.len() {
            return;
        }
        let len = self.0.len();
        if len < self.1 {
            self.0.resize(self.1, 0);
        }
        {
            let space = self.0.len() - self.1;
            let (left, right) = data.split_at(std::cmp::min(space, data.len()));
            self.0[self.1..self.1 + left.len()].copy_from_slice(left);
            self.0.extend_from_slice(right);
        }
        self.1 += data.len();
    }

    fn read_f32(&mut self) -> Option<f32> {
        if self.0.len() == 0 || self.1 + 4 > self.0.len() {
            return None;
        }
        let data = self.read(4)?;
        unsafe {
            Some(f32::from_bits(u32::from_le(
                *std::mem::transmute::<_, &u32>(data.as_ptr()),
            )))
        }
    }

    fn write_f32(&mut self, value: f32) {
        self.write(&value.to_le_bytes());
    }

    fn read_f64(&mut self) -> Option<f64> {
        if self.0.len() == 0 || self.1 + 8 > self.0.len() {
            return None;
        }
        let data = self.read(8)?;
        unsafe {
            Some(f64::from_bits(u64::from_le(
                *std::mem::transmute::<_, &u64>(data.as_ptr()),
            )))
        }
    }

    fn write_f64(&mut self, value: f64) {
        self.write(&value.to_le_bytes());
    }

    def_rw!(u8 u16 i16 u32 i32);

    fn __gc(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let _ = lua::Lcheckudata(state, 1, lua::cstr!("cslb"))
                .cast::<Self>()
                .read();
            Ok(0)
        }
    }

    fn lm_read(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            match this.read(lua::Lcheckinteger(state, 2) as _) {
                Some(val) => {
                    lua::pushlstring(state, val.as_ptr(), val.len());
                }
                None => lua::pushnil(state),
            }
            Ok(1)
        }
    }

    fn lm_write(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            this.write(check_slice!(state, 2));
            Ok(0)
        }
    }

    fn lm_read_bool(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            match this.read_u8() {
                Some(val) => {
                    lua::pushboolean(state, val as _);
                }
                None => lua::pushnil(state),
            }
            Ok(1)
        }
    }

    fn lm_write_bool(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            this.write_u8(lua::toboolean(state, 2) as _);
            Ok(0)
        }
    }

    def_lmint!(byte, u8);
    def_lmint!(ushort, u16);
    def_lmint!(short, i16);
    def_lmint!(ulong, u32);
    def_lmint!(long, i32);
    def_lmfloat!(float, f32);
    def_lmfloat!(double, f64);

    fn lm_tell(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            lua::pushinteger(state, this.1 as _);
            Ok(1)
        }
    }

    fn lm_seek(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            this.1 = (lua::Lcheckinteger(state, 2).max(0) as usize).min(this.0.len() - 1);
            Ok(0)
        }
    }

    fn lm_clear(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            this.0.clear();
            this.1 = 0;
            Ok(0)
        }
    }

    fn lm_resize(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            let size = lua::Lcheckinteger(state, 2).max(0) as usize;
            this.0.resize(size, 0);
            this.1 = this.1.min(size);
            Ok(0)
        }
    }

    fn lm_shrink(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            this.0.shrink_to_fit();
            Ok(1)
        }
    }

    fn lm_get_value(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            lua::pushlstring(state, this.0.as_ptr(), this.0.len());
            Ok(1)
        }
    }

    fn lm_set_value(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("cslb")).cast::<Self>();
            let data = check_slice!(state, 2);
            this.0.clear();
            this.1 = 0;
            this.0.extend_from_slice(data);
            Ok(0)
        }
    }

    fn metatable(state: lua_State) {
        unsafe {
            if lua::Lnewmetatable(state, lua::cstr!("cslb")) {
                lua::pushvalue(state, -1);
                lua::setfield(state, -2, lua::cstr!("__index"));
                insert_function!(state, "__gc", Self::__gc);
                insert_function!(state, "Read", Self::lm_read);
                insert_function!(state, "ReadBool", Self::lm_read_bool);
                insert_function!(state, "ReadByte", Self::lm_read_byte);
                insert_function!(state, "ReadUShort", Self::lm_read_ushort);
                insert_function!(state, "ReadShort", Self::lm_read_short);
                insert_function!(state, "ReadULong", Self::lm_read_ulong);
                insert_function!(state, "ReadLong", Self::lm_read_long);
                insert_function!(state, "ReadFloat", Self::lm_read_float);
                insert_function!(state, "ReadDouble", Self::lm_read_double);

                insert_function!(state, "Write", Self::lm_write);
                insert_function!(state, "WriteBool", Self::lm_write_bool);
                insert_function!(state, "WriteByte", Self::lm_write_byte);
                insert_function!(state, "WriteUShort", Self::lm_write_ushort);
                insert_function!(state, "WriteShort", Self::lm_write_short);
                insert_function!(state, "WriteULong", Self::lm_write_ulong);
                insert_function!(state, "WriteLong", Self::lm_write_long);
                insert_function!(state, "WriteFloat", Self::lm_write_float);
                insert_function!(state, "WriteDouble", Self::lm_write_double);

                insert_function!(state, "Tell", Self::lm_tell);
                insert_function!(state, "Seek", Self::lm_seek);
                insert_function!(state, "Clear", Self::lm_clear);
                insert_function!(state, "Resize", Self::lm_resize);
                insert_function!(state, "Shrink", Self::lm_shrink);

                insert_function!(state, "GetValue", Self::lm_get_value);
                insert_function!(state, "SetValue", Self::lm_set_value);
            }
        }
    }

    pub fn l_new(state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            let buffer = match lua::get_type(state, 1) {
                3 => Self::new(lua::tonumber(state, 1) as _),
                4 => {
                    let data = check_slice!(state, 1);
                    let mut buffer = Self::new(data.len());
                    buffer.0.extend_from_slice(data);
                    buffer
                },
                _ => {Self::new(0)}
            };
            let udata = lua::newuserdata(state, std::mem::size_of::<Self>()).cast::<Self>();
            udata.write(buffer);
            Self::metatable(state);
            lua::setmetatable(state, -2);
        }
        Ok(1)
    }
}
