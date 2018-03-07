extern crate nanomsg;
extern crate rand;
#[macro_use]
extern crate structopt;

extern crate mcp3008;

extern crate shared;

use std::thread::sleep;
use std::time::Duration;

use nanomsg::{Protocol, Socket};

use mcp3008::Mcp3008;

use rand::distributions::Range;

use structopt::StructOpt;

use shared::types::{str_to_pub_type, PubMessage};
use shared::utils::{fill_message_integral, publish, publish_random_values};

#[derive(StructOpt, Debug)]
#[structopt(name = "mcp3008_pub")]
struct Opt {
    #[structopt(short = "a", long = "adc")]
    adc: u8,

    #[structopt(short = "d", long = "delay", default_value = "1")]
    delay: u64,

    #[structopt(short = "t", long = "pub-type")]
    pub_type: String,

    #[structopt(long = "spi-dev-path", default_value = "/dev/spidev0.0")]
    spi_dev_path: String,

    #[structopt(default_value = "ipc:///tmp/mcp3008_1.ipc")]
    address: String,
}

fn main() {
    let opt = Opt::from_args();

    let pub_type =
        str_to_pub_type(&opt.pub_type).expect("invalid publisher type");

    // Create nanomsg publisher socket
    let mut socket =
        Socket::new(Protocol::Pub).expect("could not create socket");
    socket
        .bind(opt.address.as_str())
        .expect("socket bind failed");

    // Instead of creating new messages modify an existing one
    let mut msg = PubMessage {
        pub_type: pub_type,
        integral: 0,
        decimal: 0.0,
    };

    // std::thread::yield_now() can not be used to prevent excessive CPU
    // usage -> sleep 1ms instead
    let sleep_duration = Duration::from_millis(opt.delay);

    if let Ok(mut mcp3008) = Mcp3008::new(&opt.spi_dev_path) {
        loop {
            if let Ok(value) = mcp3008.read_adc(opt.adc) {
                msg = fill_message_integral(value as i16, msg);
            } else {
                panic!("could not read from Mcp3008")
            }

            publish(&mut socket, &msg);

            sleep(sleep_duration);
        }
    } else {
        publish_random_values(
            socket,
            msg,
            sleep_duration,
            Range::new(0.0, 1024.0),
        );
    }
}
