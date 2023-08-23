use rand::Rng;
use rspdk::{
    bdev::BdevDesc,
    complete::LocalComplete,
    consumer::{app_stop, SpdkConsumer},
    error::Result,
    producer::{Action, Request, SpdkProducer},
};
use std::{
    env::args,
    sync::mpsc::{channel, Receiver},
    thread,
};

fn main() {
    let (tx, rx) = channel();
    thread::spawn(|| {
        SpdkConsumer::new()
            .name("test")
            .config_file(&args().nth(1).unwrap())
            .block_on(async_main(rx))
    });

    let producer = SpdkProducer::new(tx);

    let mut rng = rand::thread_rng();

    for i in 0..10 {
        let mut buf = [0; 512];
        buf.fill(rng.gen());
        let _ = producer.produce(Action::Write, i * 512, 512, &mut buf);
        println!("write: {}", buf[0]);
        buf.fill(0);
        let _ = producer.produce(Action::Read, i * 512, 512, &mut buf);
        println!("read: {}", buf[1]);
    }
}

async fn async_main(rx: Receiver<Request>) {
    let bdev_desc = BdevDesc::create_desc("Malloc0").unwrap();

    for mut request in rx {
        let _ = match request.action {
            Action::Read => {
                bdev_desc
                    .read(request.offset, request.length, request.buf.as_mut())
                    .await
            }
            Action::Write => {
                bdev_desc
                    .write(request.offset, request.length, request.buf.as_ref())
                    .await
            }
        };
        let complete = unsafe { &mut *(request.arg as *mut LocalComplete<Result<u64>>) };
        complete.complete(Ok(0));
    }

    bdev_desc.close();
    app_stop();
}
