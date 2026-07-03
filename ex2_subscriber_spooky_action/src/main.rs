use iceoryx2::prelude::*;

#[derive(Debug, ZeroCopySend)]
#[repr(C)]
struct PayloadData {
    x: i32,
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let node = NodeBuilder::new().create::<iceoryx2::service::ipc::Service>()?;
    let service = node
        .service_builder(&"HelloIox2".try_into()?)
        .publish_subscribe::<PayloadData>()
        .open_or_create()?;
    let publisher = service.publisher_builder().create()?;

    let sub = service.subscriber_builder().create()?;

    let sample_addr = {
        let mut sample = publisher.loan_uninit()?;
        let sample_addr = sample.payload().as_ptr().addr();
        println!("All writes to {:p}", &sample.payload());
        sample.payload_mut().write(PayloadData { x: 100 });
        let sample = unsafe { sample.assume_init() };
        sample.send()?;
        sample_addr
    };

    {
        let recvd = sub.receive()?.unwrap();
        println!(
            "Received sample #1 ({:?}) at {:p}",
            recvd.payload(),
            &recvd.payload().x
        );
    }

    let mut count = 1;
    loop {
        count += 1;
        let mut sample = publisher.loan_uninit()?;
        let new_sample_addr = sample.payload().as_ptr().addr();
        sample.payload_mut().write(PayloadData { x: count * 100 });
        let sample = unsafe { sample.assume_init() };
        sample.send()?;
        if new_sample_addr == sample_addr {
            break;
        }
    }

    {
        let recvd = sub.receive()?.unwrap();
        println!(
            "Received sample #2 ({:?}) at {:p}",
            recvd.payload(),
            &recvd.payload().x
        );
    }

    Ok(())
}
