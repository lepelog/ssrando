use core::{
    alloc::Allocator,
    ffi::{c_int, c_uint, c_void},
    mem::{align_of, size_of, MaybeUninit},
    ptr::NonNull,
};

extern "C" {
    pub fn iosAllocAligned(heap: *const c_void, size: c_uint, align: c_int) -> *mut u8;
    pub fn iosFree(heap: *const c_void, ptr: *mut u8);
    pub static IOS_HEAP: *const c_void;
}

pub struct IosAllocator;

pub unsafe fn ios_allocate<'a, T>() -> &'a mut MaybeUninit<T> {
    unsafe {
        &mut *(iosAllocAligned(IOS_HEAP, size_of::<T>() as c_uint, align_of::<T>() as c_int)
            as *mut MaybeUninit<T>)
    }
}

pub unsafe fn ios_free<T>(ptr: *mut T) {
    unsafe { iosFree(IOS_HEAP, ptr as *mut u8) }
}

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
