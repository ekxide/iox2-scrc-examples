use std::{
    mem::MaybeUninit,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

#[repr(C)]
struct ShmMgmt {
    counter: AtomicUsize,
    allocator_cells: [MaybeUninit<PayloadData>; 1024],
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
struct PayloadData {
    x: i32,
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let shm_name = c"/all_glory_to_the_hypnotoad";
    let shm_size = core::mem::size_of::<ShmMgmt>();

    // create new shared memory (publisher)
    let shm_fd_publisher =
        unsafe { libc::shm_open(shm_name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o666) };
    unsafe { libc::ftruncate(shm_fd_publisher, shm_size as _) };
    let shm_base_publisher: *mut ShmMgmt = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            shm_size as _,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            shm_fd_publisher,
            0,
        )
        .cast()
    };
    unsafe {
        shm_base_publisher.write(ShmMgmt {
            counter: AtomicUsize::new(0),
            allocator_cells: [MaybeUninit::uninit(); 1024],
        })
    };

    let shm_ref_publisher: &AtomicUsize = unsafe { &(*shm_base_publisher).counter };

    // open data segment of publisher
    let shm_fd_subscriber =
        unsafe { libc::shm_open(shm_name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o666) };
    let shm_base_subscriber: *const ShmMgmt = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            shm_size as _,
            libc::PROT_READ,
            libc::MAP_SHARED,
            shm_fd_subscriber,
            0,
        )
        .cast()
    };

    let shm_ref_subscriber: &AtomicUsize = unsafe { &(*shm_base_subscriber).counter };

    for n in 0..10 {
        let payload = PayloadData { x: n };
        let write_idx = shm_ref_publisher.load(Ordering::Relaxed);
        unsafe {
            let write_ptr = &raw mut (*shm_base_publisher).allocator_cells[write_idx];
            // cast away the MaybeUninit
            write_ptr.cast::<PayloadData>().write(payload);
        }
        shm_ref_publisher.fetch_add(1, Ordering::Release);
        println!("send payload: {payload:?}");

        let read_idx = shm_ref_subscriber.load(Ordering::Acquire) - 1;
        let received = unsafe {
            // pointer is non_null and valid
            let read_pointer = &raw const (*shm_base_subscriber).allocator_cells[read_idx];
            // was initialized by the writer since the counter was at that value
            read_pointer.read().assume_init()
        };

        println!("subscriber received: {:?}", received);

        std::thread::sleep(Duration::from_millis(400));
    }

    unsafe { libc::shm_unlink(shm_name.as_ptr()) };

    Ok(())
}
