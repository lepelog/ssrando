#![no_std]
#![feature(split_array)]
#![feature(allocator_api)]
#![feature(ascii_char)]
#![feature(format_args_nl)]
#![feature(slice_ptr_get)]
#![feature(slice_partition_dedup)]
#![feature(waker_getters)]
#![feature(noop_waker)]
#![allow(dead_code)]
#![deny(clippy::no_mangle_with_rust_abi)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

extern crate alloc;

mod game;
mod rando;
mod rvl_mem;
mod rvl_os;
mod system;
mod utils;

use crate::rando::networking::{ServerProgress, CONNECTION_PORT, SOCK_STATUS};
use crate::utils::console::Console;
use core::fmt::Write;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::utils::printf::debug_print(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {{
        $crate::utils::printf::debug_print(format_args_nl!($($arg)*));
    }};
}

// Not actually mutable but needs to be to show up in custom_symbols
#[no_mangle]
#[link_section = "data"]
pub static mut SHOULD_PRINT_AP_BUFFER: bool = false;
#[no_mangle]
#[link_section = "data"]
pub static mut SHOULD_OPEN_SOCKET: bool = false;

static mut INIT_CONNECTION_TIMER: u8 = 255;

// A Common Place where Custom code can be injected to run once per frame
// Returns whether or not to stop (1 == continue)
#[no_mangle]
fn custom_main_additions() -> u32 {
    unsafe {
        if SHOULD_OPEN_SOCKET {
            if INIT_CONNECTION_TIMER == 0 {
                crate::rando::networking::run_net_init();
                INIT_CONNECTION_TIMER = 255;
            } else if !SOCK_STATUS.active {
                INIT_CONNECTION_TIMER -= 1;
            }
        }
    }
    display_socket_status();
    if unsafe { SHOULD_PRINT_AP_BUFFER } {
        return crate::rando::print_archipelago_text();
    }

    return 1;
}

fn display_socket_status() {
    let status = unsafe { &SOCK_STATUS };
    if status.last_error_code != 0 {
        let mut console = Console::with_pos(0f32, 0f32);
        console.set_bg_color(0x00000055);
        console.set_font_color(0xFFFFFFFF);
        console.set_font_size(0.5f32);
        let _ = console.write_fmt(format_args!(
            "Network code failed with error code {}",
            status.last_error_code
        ));
        match status.progress {
            ServerProgress::None => {
                let _ = console.write_str("\nCouldn't create socket, try reopening the gmae");
            },
            ServerProgress::CreatedUDP => {
                let _ = console
                    .write_str("\nCreated UDP socket but couldn't bind, try reopening the game");
            },
            ServerProgress::BoundSocket => {
                let _ = console.write_str("\nBound UDP socket, make sure AP client is working,\n then try waiting for a connection");
            },
            _ => {
                let _ = console.write_str("\nConnection was active, try waiting for a reconnect");
            },
        }
        console.draw(false);
    } else if status.active && status.progress != ServerProgress::ConnectionEstablished {
        let mut console = Console::with_pos(0f32, 0f32);
        console.set_bg_color(0x00000055);
        console.set_font_color(0xFFFFFFFF);
        console.set_font_size(0.5f32);
        let _ = console.write_fmt(format_args!(
            "Waiting for connection from AP client\nType /console {}",
            status.ip
        ));
        console.draw(false);
    }
    // else if status.num_requests > 0 {
    // let mut console = Console::with_pos(0f32, 0f32);
    // console.set_bg_color(0x00000055);
    // console.set_font_color(0xFFFFFFFF);
    // console.set_font_size(0.5f32);
    // let _ = console.write_fmt(format_args!(
    // "Received {} requests so far",
    // status.num_requests
    // ));
    // console.draw(false);
    // }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // println!("{}", info);
    loop {}
}
