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

    let shm_ref_publisher = unsafe { &mut *shm_base_publisher };

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

    let shm_ref_subscriber = unsafe { &*shm_base_subscriber };

    for n in 0..10 {
        let payload = PayloadData { x: n };
        let write_idx = shm_ref_publisher.counter.load(Ordering::Relaxed);
        shm_ref_publisher.allocator_cells[write_idx].write(payload);
        shm_ref_publisher.counter.fetch_add(1, Ordering::Release);
        println!("send payload: {payload:?}");

        let read_idx = shm_ref_subscriber.counter.load(Ordering::Acquire) - 1;

        println!("subscriber received: {:?}", unsafe {
            shm_ref_subscriber.allocator_cells[read_idx].assume_init_ref()
        });

        std::thread::sleep(Duration::from_millis(400));
    }

    unsafe { libc::shm_unlink(shm_name.as_ptr()) };

    Ok(())
}
