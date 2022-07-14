use std::ffi::c_void;
use std::hint::unreachable_unchecked;
use std::io::Cursor;
use std::io::{Read, Seek, Write};
use std::os::raw::c_uint;

use lua_shared as lua;
use lua::lua_State;

use crate::check_slice;

#[derive(Debug)]
enum KOption {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    Usize,
    Float,
    Double,
    Char,
    String,
    NOP,
}

enum Endianness {
    Little,
    Big,
    Native,
}

struct ReaderState<'a> {
    state: *mut c_void,
    endianness: Endianness,
    fmt: &'a [u8],
}

fn is_digit(byte: u8) -> bool {
    byte >= b'0' && byte <= b'9'
}

unsafe fn read_number(state: &mut ReaderState) -> Option<usize> {
    if state.fmt.len() == 0 || !is_digit(state.fmt[0]) {return None;}
    let mut result = 0;
    while state.fmt.len() > 0 && is_digit(state.fmt[0]) {
        let (opt, rest) = state.fmt.split_at(1);
        state.fmt = rest;
        result = result * 10 + (opt[0] - b'0') as usize;
    }
    Some(result)
}

unsafe fn get_option(state: &mut ReaderState) -> Option<(KOption, usize)> {
    if state.fmt.len() == 0 {
        return None;
    }
    let (opt, rest) = state.fmt.split_at(1);
    state.fmt = rest;
    match opt[0] {
        b'b' => Some((KOption::I8, std::mem::size_of::<i8>())),
        b'B' => Some((KOption::U8, std::mem::size_of::<u8>())),
        b'h' => Some((KOption::I16, std::mem::size_of::<i16>())),
        b'H' => Some((KOption::U16, std::mem::size_of::<u16>())),
        b'l' => Some((KOption::I32, std::mem::size_of::<i32>())),
        b'L' => Some((KOption::U32, std::mem::size_of::<u32>())),
        b'T' => Some((KOption::Usize, std::mem::size_of::<usize>())),
        b'f' => Some((KOption::Float, std::mem::size_of::<f32>())),
        b'd' => Some((KOption::Double, std::mem::size_of::<f64>())),
        b'n' => Some((KOption::Double, std::mem::size_of::<f64>())),
        b's' => Some((KOption::String, std::mem::size_of::<u16>())),
        b'c' => match read_number(state) {
            Some(len) => Some((KOption::Char, len)),
            None => {
                lua::Lerror(state.state, lua::cstr!("missing size for format option 'c'"));
                unreachable_unchecked();
            }
        },
        b' ' => Some((KOption::NOP, 0)),
        b'<' => {
            state.endianness = Endianness::Little;
            Some((KOption::NOP, 0))
        }
        b'>' => {
            state.endianness = Endianness::Big;
            Some((KOption::NOP, 0))
        }
        b'=' => {
            state.endianness = Endianness::Native;
            Some((KOption::NOP, 0))
        }
        token @ _ => {
            lua::Lerror(state.state, lua::cstr!("invalid format option '%c'"), token as c_uint);
            unreachable_unchecked();
        }
    }
}

// I don't fucking care. luaL_Buffer is allocated on the stack.
static mut STRING_BUFFER: [u8; 65536] = [0; 65536];

macro_rules! pack_number {
    ($state:ident, $buffer:ident, $value:tt) => {
        match $state.endianness {
            Endianness::Little => {
                $buffer.write(&$value.to_le_bytes())?;
            },
            Endianness::Big => {
                $buffer.write(&$value.to_be_bytes())?;
            },
            Endianness::Native => {
                $buffer.write(&$value.to_ne_bytes())?;
            },
        }
    };
}

macro_rules! unpack_number {
    ($state:ident, $buffer:ident, $typ:ty) => {
        {
            let mut data = [0; std::mem::size_of::<$typ>()];
            $buffer.read(&mut data)?;
            match $state.endianness {
                Endianness::Little => <$typ>::from_le_bytes(data),
                Endianness::Big => <$typ>::from_be_bytes(data),
                Endianness::Native => <$typ>::from_ne_bytes(data),
            }
        }
    };
}

pub fn pack(state: lua_State, fmt: &[u8], start: i32) -> Result<&'static [u8], std::io::Error> {
    unsafe {
        let mut arg = start;
        let mut reader_state = ReaderState {
            state: state,
            endianness: Endianness::Native,
            fmt: fmt,
        };
        let mut buffer = Cursor::new(&mut STRING_BUFFER[..]);
        while let Some((option, size)) = get_option(&mut reader_state) {
            if let KOption::NOP = option {
                continue;
            }
            if STRING_BUFFER.len() - (buffer.seek(std::io::SeekFrom::Current(0))? as usize) < size {
                lua::Lerror(state, lua::cstr!("buffer overflow"));
                unreachable_unchecked();
            }
            match option {
                KOption::I8 => {
                    let value = lua::Lcheckinteger(reader_state.state, arg) as i8;
                    pack_number!(reader_state, buffer, value);
                },
                KOption::U8 => {
                    let value = lua::Lcheckinteger(reader_state.state, arg) as u8;
                    pack_number!(reader_state, buffer, value);
                },
                KOption::I16 => {
                    let value = lua::Lcheckinteger(reader_state.state, arg) as i16;
                    pack_number!(reader_state, buffer, value);
                },
                KOption::U16 => {
                    let value = lua::Lcheckinteger(reader_state.state, arg) as u16;
                    pack_number!(reader_state, buffer, value);
                },
                KOption::I32 => {
                    let value = lua::Lcheckinteger(reader_state.state, arg) as i32;
                    pack_number!(reader_state, buffer, value);
                },
                KOption::U32 => {
                    let value = lua::Lcheckinteger(reader_state.state, arg) as u32;
                    pack_number!(reader_state, buffer, value);
                },
                KOption::Usize => {
                    let value = lua::Lcheckinteger(reader_state.state, arg) as usize;
                    pack_number!(reader_state, buffer, value);
                },
                KOption::Float => {
                    let value = lua::Lchecknumber(reader_state.state, arg) as f32;
                    pack_number!(reader_state, buffer, value);
                },
                KOption::Double => {
                    let value = lua::Lchecknumber(reader_state.state, arg);
                    pack_number!(reader_state, buffer, value);
                },
                KOption::Char => {
                    let str = check_slice!(state, arg);
                    if str.len() >= size {
                        buffer.write(&str[..size])?;
                    } else {
                        buffer.write(str)?;
                        for _ in 0..size - str.len() {
                            buffer.write(&[0])?;
                        }
                    }
                },
                KOption::String => {
                    let str = check_slice!(state, arg);
                    if str.len() > 1 << 16 {
                        lua::Lerror(state, lua::cstr!("string is too long"));
                    }
                    if (buffer.seek(std::io::SeekFrom::Current(0))? as usize) + size > STRING_BUFFER.len() {
                        lua::Lerror(state, lua::cstr!("buffer overflow"));
                    }
                    pack_number!(reader_state, buffer, (str.len() as u16));
                    buffer.write(str)?;
                },
                KOption::NOP => {},
            }
            arg += 1;
        }
        
        Ok(&STRING_BUFFER[..(buffer.seek(std::io::SeekFrom::Current(0))? as usize)])
    }
}

pub fn unpack(state: lua_State, fmt: &[u8], data: &[u8]) -> Result<i32, std::io::Error> {
    unsafe {
        let mut nrets = 0;
        let mut reader_state = ReaderState {
            state: state,
            endianness: Endianness::Native,
            fmt: fmt,
        };
        let mut buffer = Cursor::new(data);
        while let Some((option, size)) = get_option(&mut reader_state) {
            if let KOption::NOP = option {
                continue;
            }
            if data.len() - (buffer.seek(std::io::SeekFrom::Current(0))? as usize) < size {
                lua::Lerror(state, lua::cstr!("not enough data in the buffer"));
                unreachable_unchecked();
            }
            nrets += 1;
            match option {
                KOption::I8 => {
                    let value = unpack_number!(reader_state, buffer, i8);
                    lua::pushinteger(state, value as _);
                },
                KOption::U8 => {
                    let value = unpack_number!(reader_state, buffer, u8);
                    lua::pushinteger(state, value as _);
                },
                KOption::I16 => {
                    let value = unpack_number!(reader_state, buffer, i16);
                    lua::pushinteger(state, value as _);
                },
                KOption::U16 => {
                    let value = unpack_number!(reader_state, buffer, u16);
                    lua::pushinteger(state, value as _);
                },
                KOption::I32 => {
                    let value = unpack_number!(reader_state, buffer, i32);
                    lua::pushinteger(state, value as _);
                },
                KOption::U32 => {
                    let value = unpack_number!(reader_state, buffer, u32);
                    lua::pushinteger(state, value as _);
                },
                KOption::Usize => {
                    let value = unpack_number!(reader_state, buffer, usize);
                    lua::pushinteger(state, value as _);
                },
                KOption::Float => {
                    let value = unpack_number!(reader_state, buffer, f32);
                    lua::pushnumber(state, value as _);
                },
                KOption::Double => {
                    let value = unpack_number!(reader_state, buffer, f64);
                    lua::pushnumber(state, value);
                },
                KOption::Char => {
                    let offset = buffer.seek(std::io::SeekFrom::Current(0))? as usize;
                    lua::pushlstring(state, data.as_ptr().add(offset), size);
                },
                KOption::String => {
                    let value = unpack_number!(reader_state, buffer, u16);
                    let offset = buffer.seek(std::io::SeekFrom::Current(0))? as usize;
                    if offset + value as usize > data.len() {
                        lua::Lerror(state, lua::cstr!("not enough data in the buffer"));
                    }
                    lua::pushlstring(state, data.as_ptr().add(offset), value as _);
                },
                KOption::NOP => {},
            }
        }
        Ok(nrets)
    }
}