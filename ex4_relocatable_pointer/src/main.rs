use iceoryx2::prelude::*;
use iceoryx2_bb_container::queue::FixedSizeQueue;

#[derive(Debug, ZeroCopySend)]
#[repr(C)]
struct PayloadData {
    q: FixedSizeQueue<i32, 10>,
}

impl PayloadData {
    fn print_queue(&self) {
        if self.q.is_empty() {
            println!("Queue is empty");
        } else {
            println!("Element in queue is {}", self.q.peek().unwrap());
        }
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

    let mut q = FixedSizeQueue::new();
    q.push(42);
    q.push(43);
    q.push(44);
    publisher.send_copy(PayloadData { q })?;

    let recvd = sub.receive()?.unwrap();
    println!("Received payload object.");
    recvd.payload().print_queue();

    Ok(())
}
