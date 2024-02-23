use core::ffi::{c_int, c_void};

extern "C" {
    fn OSLockMutex(ptr: *const c_void);
    fn OSUnlockMutex(ptr: *const c_void);
    fn OSTryLockMutex(ptr: *const c_void) -> c_int;
}

#[repr(C, align(4))]
pub struct WiiMutex {
    data: [u8; 24],
}

impl WiiMutex {
    pub const fn new() -> Self {
        // OSInitMutex also only sets stuff to 0
        WiiMutex { data: [0; 24] }
    }
    fn as_c_void(&self) -> *const c_void {
        self as *const WiiMutex as *const c_void
    }
    pub fn lock(&self) -> WiiLockGuard<'_> {
        unsafe {
            OSLockMutex(self.as_c_void());
        }
        WiiLockGuard { mutex: self }
    }
    pub fn try_lock(&self) -> Option<WiiLockGuard<'_>> {
        let result = unsafe { OSTryLockMutex(self.as_c_void()) };
        if result == 1 {
            Some(WiiLockGuard { mutex: self })
        } else {
            None
        }
    }
}

pub struct WiiLockGuard<'a> {
    mutex: &'a WiiMutex,
}

impl<'a> Drop for WiiLockGuard<'a> {
    fn drop(&mut self) {
        unsafe {
            OSUnlockMutex(self.mutex.as_c_void());
        }
    }
}
