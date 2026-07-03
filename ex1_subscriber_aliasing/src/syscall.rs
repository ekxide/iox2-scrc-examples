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

    // create new shared memory (publisher)
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

    unsafe {
        shm_base_publisher
            .add(offset)
            .cast::<PayloadData>()
            .write(payload)
    };
    println!("send payload: {payload:?}");

    // open data segment of publisher (subscriber 1)
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

    println!("subscriber 1 received: {:?}", unsafe {
        &*shm_base_subscriber_1.add(offset).cast::<PayloadData>()
    });

    // open data segment of publisher (subscriber 2)
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

    println!("subscriber 2 received: {:?}", unsafe {
        &*shm_base_subscriber_2.add(offset).cast::<PayloadData>()
    });

    unsafe { libc::shm_unlink(shm_name.as_ptr()) };

    Ok(())
}
