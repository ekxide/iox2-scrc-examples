#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct PayloadData {
    x: i32,
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let shm_name = c"/hypnotoad";
    let shm_size = 4096 * 10;
    let offset = 128;
    let payload = PayloadData { x: 123098 };

    // In Process 1: create new shared memory (publisher)
    // `Publisher::create()`
    let shm_fd_publisher =
        unsafe { libc::shm_open(shm_name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o666) };
    unsafe { libc::ftruncate(shm_fd_publisher, shm_size) };
    let shm_base_publisher = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            shm_size as _,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            shm_fd_publisher,
            0,
        )
    };

    // In Process 2: open data segment of publisher (subscriber 1)
    // `Subscriber::open()`
    let shm_fd_subscriber_1 =
        unsafe { libc::shm_open(shm_name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o666) };
    let shm_base_subscriber_1 = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            shm_size as _,
            libc::PROT_READ,
            libc::MAP_SHARED,
            shm_fd_subscriber_1,
            0,
        )
    };

    // In Process 3: open data segment of publisher (subscriber 2)
    let shm_fd_subscriber_2 =
        unsafe { libc::shm_open(shm_name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o666) };
    let shm_base_subscriber_2 = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            shm_size as _,
            libc::PROT_READ,
            libc::MAP_SHARED,
            shm_fd_subscriber_2,
            0,
        )
    };

    // Publisher writing data:
    // `Publisher::loan_uninit()`
    let sample_mut: *mut PayloadData =
        unsafe { shm_base_publisher.add(offset).cast::<PayloadData>() };

    // `SampleMutUninit::write_payload()`
    unsafe { sample_mut.write(payload) };

    // `SampleMut::send()` writes the offset to the payload in an internal lock-free queue
    // read-only sample is still available on sender side
    let sample: *const PayloadData = sample_mut.cast_const();
    drop(sample_mut);
    println!("send payload: {payload:?}");

    // Subscribers receiving data:
    // `Subscriber_1::receive()`;
    let sample_1: *const PayloadData =
        unsafe { shm_base_subscriber_1.add(offset).cast::<PayloadData>() };

    println!("subscriber 1 received: {:?}", unsafe { &*sample_1 });

    // `Subscriber_2::receive()`;
    let sample_2: *const PayloadData =
        unsafe { shm_base_subscriber_2.add(offset).cast::<PayloadData>() };
    println!("subscriber 2 received: {:?}", unsafe { &*sample_2 });

    unsafe { libc::shm_unlink(shm_name.as_ptr()) };

    Ok(())
}
