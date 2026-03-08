pub mod alarm;
pub mod button;
pub mod gx;
pub mod heap;
pub mod ios;
pub mod math;
pub mod mutex;
pub mod os;
pub mod ppc;
pub mod time;

extern "C" {
    static mut GAME_FRAME: u32;
}

pub fn game_frame() -> u32 {
    unsafe { GAME_FRAME }
}
