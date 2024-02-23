extern "C" {
    fn OSGetTick() -> u32;
    pub fn OSRegisterShutdownFunction(cb: extern "C" fn());
}

pub fn get_time_base() -> u32 {
    let reg = unsafe { (0x800000F8 as *const u32).read_volatile() };
    reg / 4000
}

pub fn os_get_tick() -> u32 {
    unsafe { OSGetTick() }
}
