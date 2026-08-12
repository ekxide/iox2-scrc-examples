use std::sync::atomic::{AtomicU64, Ordering};

#[repr(C)]
struct ShmMgmt {
    init: AtomicU64,
    data: [u8; 1024],
}

fn main() {
    let shm_name = c"/all_glory_to_the_hypnotoad";
    let shm_size = core::mem::size_of::<ShmMgmt>();

    // create shm segment
    let shm_creator =
        unsafe { libc::shm_open(shm_name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o666) };
    unsafe { libc::ftruncate(shm_creator, shm_size as _) };
    let shm_creator: *mut ShmMgmt = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            shm_size as _,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            shm_creator,
            0,
        )
        .cast()
    };

    // initialize shm segment
    let shm_creator_ref = unsafe { &mut *shm_creator };
    let init_ptr = core::ptr::addr_of_mut!(shm_creator_ref.init);
    // this write races with the read in the wait loop further down below;
    // there is an implicit assumption here that the value the init_ptr points
    // to will not randomly be equal to the magic number 5555;
    // in practice this is achieved through external guarantees from the
    // underlying operating system
    unsafe { init_ptr.write(AtomicU64::new(0)) };
    let data_ptr = core::ptr::addr_of_mut!(shm_creator_ref.data);
    unsafe { data_ptr.write([0u8; 1024]) };

    // set shm segment to: is initialized
    unsafe { (&*init_ptr).store(5555, Ordering::Release) };

    // open shm segment
    let shm_opener =
        unsafe { libc::shm_open(shm_name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o666) };
    let shm_opener: *mut ShmMgmt = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            shm_size as _,
            libc::PROT_READ,
            libc::MAP_SHARED,
            shm_opener,
            0,
        )
        .cast()
    };

    // wait for initialization
    // this may execute concurrently to the initialization above
    let shm_opener_ref = unsafe { &mut *shm_opener };
    let init_ptr = core::ptr::addr_of_mut!(shm_opener_ref.init);
    // this read will trigger a benign read access to uninitialized memory
    // - first, the memory location that init_ptr points to was never
    //   initialized, only an aliasing location that happens to point to the
    //   same physical address
    // - second, there is no way to get rid of the race with the first write
    //   performing the initialization above; it cannot be guaranteed that this
    //   load will not end up reading the memory before it was written by any
    //   user code
    while unsafe { &*init_ptr }.load(Ordering::Acquire) != 5555 {}

    unsafe { libc::shm_unlink(shm_name.as_ptr()) };
}
