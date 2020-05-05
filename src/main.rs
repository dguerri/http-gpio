use serde::{Serialize, Deserialize};
use warp::{Filter, http::StatusCode};
use gpio_cdev::{Chip, LineHandle, LineRequestFlags};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

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

lazy_static! {
    static ref HASHMAP: Mutex<HashMap<String, LineHandle>> = Mutex::new(HashMap::new());
}

fn gpio_modify(chip_name: String, pin: u32, body: GpioCmd) -> GpioModifyResult {
    let mut hm = HASHMAP.lock().unwrap();
    let line_handle_name = format!("{}_{}_{}", chip_name, pin, "out");

    let line_hadle = match hm.get_mut(&line_handle_name) {
        None => {
            let mut c = Chip::new(format!("/dev/{}", chip_name))?;
            let lh = c
                .get_line(pin)?
                .request(LineRequestFlags::OUTPUT, 0, "http-gpio")?;
            hm.insert(line_handle_name.clone(), lh);
            hm.get_mut(&line_handle_name).unwrap()
        }
        Some(lh) => lh,
    };

    match body {
        GpioCmd::Out { value } => {
            line_hadle.set_value(value as u8)?;
            Ok(None)
        }
        GpioCmd::In => Ok(Some(line_hadle.get_value()?)),
    }
}

fn as_reply(value: GpioModifyResult) -> Box<dyn warp::Reply> {
    // Return if success, or stringify the error if not
    match value {
        Ok(Some(value)) => Box::new(format!("Success, value: {}", value)),
        Ok(None) => Box::new("Success"),
        Err(err) => Box::new(warp::reply::with_status(
            err.to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

