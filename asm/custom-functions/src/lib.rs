#![no_std]
#![feature(split_array)]
#![feature(allocator_api)]
#![feature(ip_in_core)]
#![allow(dead_code)]
#![deny(clippy::no_mangle_with_rust_abi)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

use cstr::cstr;
use rvl_os::ss_printf;

mod game;
mod rando;
mod rvl_mem;
mod rvl_mutex;
mod rvl_os;
mod system;
mod utils;

pub fn console_print(args: core::fmt::Arguments<'_>) {
    use core::fmt::Write;
    let mut s = arrayvec::ArrayString::<512>::new();
    let _ = s.write_fmt(args);
    let _ = s.try_push('\0');
    unsafe {
        // this might break the string if the last character is a multibyte
        // character, but that probably never happens
        if let Some(last) = s.as_bytes_mut().last_mut() {
            *last = 0;
        }
    }
    unsafe {
        ss_printf(cstr!("%s").as_ptr(), s.as_bytes().as_ptr());
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
