extern "C" {

    fn OSDisableInterrupts() -> u32;
    fn OSRestoreInterrupts(restore: u32);
}

pub fn with_disabled_interrupts<T>(f: impl FnOnce() -> T) -> T {
    let state = unsafe { OSDisableInterrupts() };
    let result = f();
    unsafe { OSRestoreInterrupts(state) };
    result
}
