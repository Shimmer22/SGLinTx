//! This crate contains Linux joydev definitions from `linux/joystick.h`.
//!
//! Those are raw definitions so for documentation see the official
//! [kernel documentation](https://www.kernel.org/doc/html/latest/input/joydev/joystick-api.html#joystick-api).
#![cfg(target_os = "linux")]
#![no_std]

extern crate input_event_codes;
extern crate libc;
extern crate nix;

use core::mem::size_of;

use input_event_codes::{ABS_CNT, BTN_MISC, KEY_CNT};
use libc::{__s16, __s32, __u16, __u32, __u8, c_long, c_longlong, c_ulong, size_t};
use nix::{request_code_read, request_code_write};

#[allow(non_camel_case_types)]
pub type __s64 = c_longlong;

pub const JS_VERSION: __u32 = 0x0002_0100;

pub const JS_EVENT_BUTTON: __u8 = 0x01;
pub const JS_EVENT_AXIS: __u8 = 0x02;
pub const JS_EVENT_INIT: __u8 = 0x80;

#[allow(non_camel_case_types)]
#[cfg_attr(feature = "extra_traits", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct js_event {
	pub time: __u32,
	pub value: __s16,
	pub type_: __u8,
	pub number: __u8,
}

impl Copy for js_event {}

impl Clone for js_event {
	fn clone(&self) -> Self {
		*self
	}
}

pub const JSIOCGVERSION: libc::c_int = request_code_read!(b'j', 0x01, size_of::<__u32>()) as libc::c_int;

pub const JSIOCGAXES: libc::c_int = request_code_read!(b'j', 0x11, size_of::<__u8>()) as libc::c_int;
pub const JSIOCGBUTTONS: libc::c_int = request_code_read!(b'j', 0x12, size_of::<__u8>()) as libc::c_int;
#[allow(non_snake_case)]
pub const fn JSIOCGNAME(len: size_t) -> libc::c_int {
	request_code_read!(b'j', 0x13, len) as libc::c_int
}

pub const JSIOCSCORR: libc::c_int = request_code_write!(b'j', 0x21, size_of::<js_corr>()) as libc::c_int;
pub const JSIOCGCORR: libc::c_int = request_code_read!(b'j', 0x22, size_of::<js_corr>()) as libc::c_int;

pub const JSIOCSAXMAP: libc::c_int = request_code_write!(b'j', 0x31, size_of::<[__u8; ABS_CNT as usize]>()) as libc::c_int;
pub const JSIOCGAXMAP: libc::c_int = request_code_read!(b'j', 0x32, size_of::<[__u8; ABS_CNT as usize]>()) as libc::c_int;
pub const JSIOCSBTNMAP: libc::c_int = request_code_write!(b'j', 0x33, size_of::<[__u16; (KEY_CNT - BTN_MISC) as usize]>()) as libc::c_int;
pub const JSIOCGBTNMAP: libc::c_int = request_code_read!(b'j', 0x34, size_of::<[__u16; (KEY_CNT - BTN_MISC) as usize]>()) as libc::c_int;

pub const JS_CORR_NONE: __u16 = 0x00;
pub const JS_CORR_BROKEN: __u16 = 0x01;

#[allow(non_camel_case_types)]
#[cfg_attr(feature = "extra_traits", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct js_corr {
	pub coef: [__s32; 8],
	pub prec: __s16,
	pub type_: __u16,
}

impl Copy for js_corr {}

impl Clone for js_corr {
	fn clone(&self) -> Self {
		*self
	}
}

pub const JS_RETURN: size_t = size_of::<JS_DATA_TYPE>();
pub const JS_TRUE: __u8 = 1;
pub const JS_FALSE: __u8 = 0;
pub const JS_X_0: __u8 = 0x01;
pub const JS_Y_0: __u8 = 0x02;
pub const JS_X_1: __u8 = 0x04;
pub const JS_Y_1: __u8 = 0x08;
pub const JS_MAX: __u8 = 2;

pub const JS_DEF_TIMEOUT: __u16 = 0x1300;
pub const JS_DEF_CORR: __u8 = 0;
pub const JS_DEF_TIMELIMIT: c_long = 10;

pub const JS_SET_CAL: __u8 = 1;
pub const JS_GET_CAL: __u8 = 2;
pub const JS_SET_TIMEOUT: __u8 = 3;
pub const JS_GET_TIMEOUT: __u8 = 4;
pub const JS_SET_TIMELIMIT: __u8 = 5;
pub const JS_GET_TIMELIMIT: __u8 = 6;
pub const JS_GET_ALL: __u8 = 7;
pub const JS_SET_ALL: __u8 = 8;

#[allow(non_camel_case_types)]
#[cfg_attr(feature = "extra_traits", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct JS_DATA_TYPE {
	pub buttons: __s32,
	pub x: __s32,
	pub y: __s32,
}

impl Copy for JS_DATA_TYPE {}

impl Clone for JS_DATA_TYPE {
	fn clone(&self) -> Self {
		*self
	}
}

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[cfg_attr(feature = "extra_traits", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct JS_DATA_SAVE_TYPE_32 {
	pub JS_TIMEOUT: __s32,
	pub BUSY: __s32,
	pub JS_EXPIRETIME: __s32,
	pub JS_TIMELIMIT: __s32,
	pub JS_SAVE: JS_DATA_TYPE,
	pub JS_CORR: JS_DATA_TYPE,
}

impl Copy for JS_DATA_SAVE_TYPE_32 {}

impl Clone for JS_DATA_SAVE_TYPE_32 {
	fn clone(&self) -> Self {
		*self
	}
}

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[cfg_attr(feature = "extra_traits", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct JS_DATA_SAVE_TYPE_64 {
	pub JS_TIMEOUT: __s32,
	pub BUSY: __s32,
	pub JS_EXPIRETIME: __s64,
	pub JS_TIMELIMIT: __s64,
	pub JS_SAVE: JS_DATA_TYPE,
	pub JS_CORR: JS_DATA_TYPE,
}

impl Copy for JS_DATA_SAVE_TYPE_64 {}

impl Clone for JS_DATA_SAVE_TYPE_64 {
	fn clone(&self) -> Self {
		*self
	}
}
