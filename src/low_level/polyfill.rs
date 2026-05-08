pub unsafe fn transmute_unchecked<Src, Dst>(src: Src) -> Dst {
    let dst: *const Src = &src;
    let dst = dst.cast();
    let dst = unsafe { ::core::ptr::read(dst) };
    ::core::mem::forget(src);
    dst
}
