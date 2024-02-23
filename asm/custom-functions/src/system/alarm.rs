#[repr(C)]
pub struct OSAlarm {
    // treat data as opaque for now
    data: [u8; 44],
}

impl OSAlarm {
    pub const fn new() -> Self {
        OSAlarm { data: [0; 44] }
    }
}

extern "C" {
    pub fn OSSetPeriodicAlarm(
        alarm: *mut OSAlarm,
        start: u64,
        period: u64,
        callback: extern "C" fn(*mut OSAlarm),
    );
    pub fn OSInsertAlarm(alarm: *mut OSAlarm, wait: u64, callback: extern "C" fn(*mut OSAlarm));
    fn OSGetTick() -> u32;
}
