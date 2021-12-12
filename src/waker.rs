use core::ptr;
use core::task::{RawWaker, RawWakerVTable, Waker};

unsafe fn waker_clone(_: *const ()) -> RawWaker {
    null_raw()
}

unsafe fn nop(_: *const ()) {}

const NULL_WAKER_TABLE: RawWakerVTable = RawWakerVTable::new(waker_clone, nop, nop, nop);

pub(crate) fn null_raw() -> RawWaker {
    RawWaker::new(ptr::null(), &NULL_WAKER_TABLE)
}

pub(crate) fn null() -> Waker {
    unsafe { Waker::from_raw(null_raw()) }
}
