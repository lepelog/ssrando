use core::ffi::{c_char, c_int, c_void};

extern "C" {
    pub fn IOS_Open(path: *const c_char, mode: c_int) -> c_int;
    pub fn IOS_OpenAsync(
        path: *const c_char,
        mode: c_int,
        callback: extern "C" fn(c_int, *mut c_void),
        userdata: *mut c_void,
    ) -> c_int;
    pub fn IOS_Ioctlv(
        fd: c_int,
        cmd: c_int,
        in_cnt: c_int,
        out_cnt: c_int,
        ioctlv: *mut c_void,
    ) -> c_int;
    pub fn IOS_IoctlvAsync(
        fd: c_int,
        cmd: c_int,
        in_cnt: c_int,
        out_cnt: c_int,
        ioctlv: *mut c_void,
        callback: extern "C" fn(c_int, *mut c_void),
        userdata: *mut c_void,
    ) -> c_int;
    pub fn IOS_Ioctl(
        fd: c_int,
        cmd: c_int,
        in_buf: *mut c_void,
        in_len: c_int,
        out_buf: *mut c_void,
        out_len: c_int,
    ) -> c_int;
    pub fn IOS_IoctlAsync(
        fd: c_int,
        cmd: c_int,
        in_buf: *mut c_void,
        in_len: c_int,
        out_buf: *mut c_void,
        out_len: c_int,
        callback: extern "C" fn(c_int, *mut c_void),
        userdata: *mut c_void,
    ) -> c_int;
    pub fn IOS_Close(fd: c_int) -> c_int;
    pub fn IOS_CloseAsync(
        fd: c_int,
        callback: extern "C" fn(c_int, *mut c_void),
        userdata: *mut c_void,
    ) -> c_int;
}
