use gpio_cdev::{Chip, LineRequestFlags};
use std::{thread, time};
use serde::{Serialize, Deserialize};
use warp::{Filter, http::StatusCode};

#[tokio::main]
async fn main() {
    let gpio_hello = warp::path!("gpio")
        .map(|| "This is the GPIO API");

    let gpio_modify = warp::post()
        .and(warp::path!("gpio" / String / u32))
        /* 1KB should be enough for anyone */
        .and(warp::body::content_length_limit(1024))
        .and(warp::body::json())
        .map(gpio_modify)
        .map(as_reply);

    let routes = gpio_hello.or(gpio_modify);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

#[derive(Serialize,Deserialize,Debug)]
enum GpioCmd {
    In,
    Out {
        value: bool,
    },
}

type GpioModifyResult = Result<Option<u8>, gpio_cdev::errors::Error>;

fn gpio_modify(chip: String, pin: u32, body: GpioCmd) -> GpioModifyResult {
    let line = Chip::new(format!("/dev/{}", chip))?.get_line(pin)?;
    match body {
        GpioCmd::Out { value } => {
            // We need to keep the handle in scope
            // see https://github.com/rust-embedded/gpio-cdev/issues/29
            let handle = line.request(LineRequestFlags::OUTPUT, 0, "http-gpio")?;
            handle.set_value(value as u8)?;
            thread::sleep(time::Duration::from_secs(1));
            Ok(None)
        }
        GpioCmd::In => {
            let handle = line.request(LineRequestFlags::INPUT, 0, "http-gpio")?;
            Ok(Some(handle.get_value()?))
        }
    }
}

fn as_reply(value: GpioModifyResult) -> Box<dyn warp::Reply> {
    // Return if success, or stringify the error if not
    match value {
        Ok(None) => Box::new("Success"),
        Ok(Some(value)) => Box::new(format!("Success, value: {}", value)),
        Err(err) => Box::new(
            warp::reply::with_status(err.to_string(),
                                     StatusCode::INTERNAL_SERVER_ERROR))
    }
}
