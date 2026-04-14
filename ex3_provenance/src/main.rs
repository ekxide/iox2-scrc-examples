use iceoryx2::prelude::*;

#[derive(Debug, ZeroCopySend)]
#[repr(C)]
struct PayloadData {
    x: i32,
}

impl PayloadData {
    fn print_x(&self) {
        println!("X is {}", self.x)
    }
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let node = NodeBuilder::new().create::<iceoryx2::service::ipc::Service>()?;
    let service = node
        .service_builder(&"HelloIox2".try_into()?)
        .publish_subscribe::<PayloadData>()
        .open_or_create()?;
    let publisher = service.publisher_builder().create()?;

    let sub = service.subscriber_builder().create()?;

    publisher.send_copy(PayloadData { x: 42 })?;

    let recvd = sub.receive()?.unwrap();
    println!("Received payload object.");
    recvd.payload().print_x();

    Ok(())
}
