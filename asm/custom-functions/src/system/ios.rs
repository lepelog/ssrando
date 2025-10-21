use core::{
    alloc::Allocator,
    ffi::{c_char, c_int, c_uint, c_void},
    ptr::NonNull,
};

extern "C" {
    pub fn iosAllocAligned(heap: *const c_void, size: c_uint, align: c_int) -> *mut u8;
    pub fn iosFree(heap: *const c_void, ptr: *mut u8);
    pub static IOS_HEAP: *const c_void;
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

pub struct IosAllocator;

unsafe impl Allocator for IosAllocator {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        let ptr =
            unsafe { iosAllocAligned(IOS_HEAP, layout.size() as c_uint, layout.align() as c_int) };
        let ret = core::ptr::NonNull::new(ptr).ok_or(core::alloc::AllocError)?;
        Ok(NonNull::slice_from_raw_parts(ret, layout.size()))
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        iosFree(IOS_HEAP, ptr.as_ptr())
    }

    fn allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, core::alloc::AllocError> {
        // the default is already zero allocating
        self.allocate(layout)
    }
}
