#![no_std]
#![feature(split_array)]
#![feature(allocator_api)]
#![feature(ip_in_core)]
#![feature(waker_getters)]
#![feature(noop_waker)]
#![allow(dead_code)]
#![deny(clippy::no_mangle_with_rust_abi)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

use core::alloc::GlobalAlloc;

use cstr::cstr;
use rvl_os::ss_printf;

extern crate alloc;

#[global_allocator]
static DUMMY_ALLOC: PanicAlloc = PanicAlloc;

mod game;
mod rando;
mod rvl_mem;
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

struct PanicAlloc;

unsafe impl GlobalAlloc for PanicAlloc {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        console_print(format_args!("don't use the default allocator!\n"));
        panic!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        panic!()
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
