#[macro_export]
macro_rules! check_slice {
    ($state:ident, $index:expr) => {{
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

#[macro_export]
macro_rules! tree_get_key {
    ($name:ident, $udata:expr) => {
        paste::paste! {
            fn [<lm_ $name>](state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
                unsafe {
                    let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
                    let key = check_slice!(state, 2);
                    let lt = this.$name(key)?;
                    if let Some((key, value)) = lt {
                        lua::pushlstring(state, key.as_ptr(), key.len());
                        lua::pushlstring(state, value.as_ptr(), value.len());
                        Ok(2)
                    } else {
                        Ok(0)
                    }
                }
            }

            fn [<lm_ $name _struct>](state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
                unsafe {
                    let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!("csldb")).cast::<Self>();
                    let key = check_slice!(state, 2);
                    let fmt = check_slice!(state, 3);
                    if let Some((key, value)) = this.$name(key)? {
                        lua::pushlstring(state, key.as_ptr(), key.len());
                        match lua_struct::unpack(state, fmt, &value) {
                            Ok(args) => Ok(args + 1),
                            Err(e) => {
                                drop(key);
                                drop(value);
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
        }
    };
    ($($name:ident )+, $udata:expr) => {
        $(tree_get_key!($name, $udata);)+
    };
}

#[macro_export]
macro_rules! tree_get_no_arg {
    ($name:ident, $udata:expr) => {
        paste::paste! {
            fn [<lm_ $name>](state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
                unsafe {
                    let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!($udata)).cast::<Self>();
                    if let Some((key, value)) = this.$name()? {
                        lua::pushlstring(state, key.as_ptr(), key.len());
                        lua::pushlstring(state, value.as_ptr(), value.len());
                        Ok(2)
                    } else {
                        Ok(0)
                    }
                }
            }

            fn [<lm_ $name _struct>](state: lua_State) -> Result<i32, Box<dyn std::error::Error>> {
                unsafe {
                    let this = &mut *lua::Lcheckudata(state, 1, lua::cstr!($udata)).cast::<Self>();
                    let fmt = check_slice!(state, 2);
                    if let Some((key, value)) = this.$name()? {
                        lua::pushlstring(state, key.as_ptr(), key.len());
                        match lua_struct::unpack(state, fmt, &value) {
                            Ok(args) => Ok(args + 1),
                            Err(e) => {
                                drop(key);
                                drop(value);
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
        }
    };
    ($($name:ident )+, $udata:expr) => {
        $(tree_get_no_arg!($name, $udata);)+
    };
}
