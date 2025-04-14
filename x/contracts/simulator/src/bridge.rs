use crate::bindings::{Bytes, BytesWithError};
use crate::state::{GetStateCallback, InsertStateCallback, RemoveStateCallback, SimpleState};

#[no_mangle]
pub extern "C" fn bridge_get_callback(
    get_func: GetStateCallback,
    state_ptr: *mut SimpleState,
    key: Bytes,
) -> BytesWithError {
    // Safety: state_ptr is a valid pointer to a SimpleState
    let state = unsafe { &mut *state_ptr };
    get_func(state, key)
}

#[no_mangle]
pub extern "C" fn bridge_insert_callback(
    insert_func: InsertStateCallback,
    state_ptr: *mut SimpleState,
    key: Bytes,
    value: Bytes,
) {
    // Safety: state_ptr is a valid pointer to a SimpleState
    let state = unsafe { &mut *state_ptr };
    insert_func(state, key, value)
}

#[no_mangle]
pub extern "C" fn bridge_remove_callback(
    remove_func: RemoveStateCallback,
    state_ptr: *mut SimpleState,
    key: Bytes,
) {
    // Safety: state_ptr is a valid pointer to a SimpleState
    let state = unsafe { &mut *state_ptr };
    remove_func(state, key)
}
