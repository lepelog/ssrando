//! for referencing another actor

use core::{
    ffi::c_void,
    marker::PhantomData,
    ptr::{null_mut, NonNull},
};

#[repr(C)]
struct RawActorReference {
    prev: *mut c_void,
    next: *mut c_void,
    link: *mut c_void,
}

extern "C" {
    fn ActorReference__unlink(this: *mut RawActorReference);
    fn ActorReference__link(this: *mut RawActorReference, actor: *mut c_void);
}

// TODO: should require T: Actor
pub struct ActorReference<T> {
    raw: RawActorReference,
    _p:  PhantomData<T>,
}

impl<T> ActorReference<T> {
    pub fn new() -> Self {
        Self {
            raw: RawActorReference {
                prev: null_mut(),
                next: null_mut(),
                link: null_mut(),
            },
            _p:  PhantomData,
        }
    }

    fn get_ptr_mut(&mut self) -> *mut RawActorReference {
        (&mut self.raw) as *mut RawActorReference
    }

    pub fn link(&mut self, other: *mut T) {
        unsafe {
            ActorReference__link(self.get_ptr_mut(), other.cast());
        }
    }

    pub fn unlink(&mut self) {
        unsafe {
            ActorReference__unlink(self.get_ptr_mut());
        }
    }

    pub fn get(&self) -> Option<NonNull<T>> {
        NonNull::new(self.raw.link.cast())
    }
}

impl<T> Drop for ActorReference<T> {
    fn drop(&mut self) {
        self.unlink();
    }
}
