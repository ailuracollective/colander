//! The caller allocator.
//!
//! A caller that cannot allocate in this library's address space has no way to
//! build the NUL-terminated request the entry points take. These two symbols
//! give it one.
//!
//! They are the only allocation outside [`colander_free_string`], and they exist
//! because output ownership is not enough: the caller also has to *produce* the
//! input. Ask for `byte_length + 1`, write the text, put a NUL at
//! `byte_length`, and release with the same length you asked for.
//!
//! [`colander_free_string`]: super::envelope::colander_free_string

/// Allocate `length` bytes for a request.
///
/// Returns null when `length` is zero or the layout is not representable, so a
/// non-null result is always safe to write to.
#[unsafe(no_mangle)]
pub extern "C" fn colander_alloc(length: usize) -> *mut u8 {
    if length == 0 {
        return std::ptr::null_mut();
    }
    let Ok(layout) = std::alloc::Layout::array::<u8>(length) else {
        return std::ptr::null_mut();
    };
    // SAFETY: the layout is valid by construction and non-zero.
    unsafe { std::alloc::alloc(layout) }
}

/// Release a buffer from [`colander_alloc`].
///
/// The same `length` that was requested must be passed back: the pair is the
/// whole contract, and there is no way to recover the size from the pointer.
///
/// # Safety
/// `pointer` must come from [`colander_alloc`] with this exact `length`, and must
/// not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colander_free_buffer(pointer: *mut u8, length: usize) {
    if pointer.is_null() || length == 0 {
        return;
    }
    let Ok(layout) = std::alloc::Layout::array::<u8>(length) else {
        return;
    };
    // SAFETY: the caller's contract is that this pointer came from
    // `std::alloc::alloc` with this same layout.
    unsafe { std::alloc::dealloc(pointer, layout) };
}
