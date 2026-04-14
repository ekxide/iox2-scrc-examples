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
    let publisher = service
        .publisher_builder()
        .create()?;

    let sub1 = service.subscriber_builder().create()?;
    let sub2 = service.subscriber_builder().create()?;

    let mut sample = publisher.loan_uninit()?;
    sample.payload_mut().write(PayloadData { x: 100 });
    let sample = unsafe { sample.assume_init() };
    sample.send()?;

    let recvd1 = sub1.receive()?.unwrap();
    let recvd2 = sub2.receive()?.unwrap();

    println!(
        "Sample #1 ({:?}) at {:p}",
        recvd1.payload(),
        &recvd1.payload().x
    );
    println!(
        "Sample #2 ({:?}) at {:p}",
        recvd2.payload(),
        &recvd2.payload().x
    );

    Ok(())
}
