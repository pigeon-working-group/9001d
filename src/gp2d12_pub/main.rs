extern crate mcp3008;
extern crate nanomsg;
extern crate rand;
#[macro_use]
extern crate structopt;

extern crate colored;

extern crate quickersort;

extern crate gp2d12;
extern crate shared;

use std::thread::sleep;
use std::time::Duration;

use nanomsg::{Protocol, Socket};

use mcp3008::Mcp3008;

use structopt::StructOpt;

use gp2d12::Gp2d12;

use rand::distributions::Range;

use shared::types::{PubMessage, PubType};
use shared::utils::{fill_message_decimal, publish, publish_random_values};

const VALUE_BUFFER_SIZE: usize = 25;
const EXCLUSION_RANGE: usize = VALUE_BUFFER_SIZE / 5;
const LOGICAL_BOXPLOT_SIZE: f32 =
    (VALUE_BUFFER_SIZE - (EXCLUSION_RANGE * 2)) as f32;

#[derive(StructOpt, Debug)]
#[structopt(name = "gp2d12_pub")]
struct Opt {
    #[structopt(short = "a", long = "adc")]
    adc: u8,

    #[structopt(short = "d", long = "delay", default_value = "1")]
    delay: u64,

    #[structopt(long = "spi-dev-path", default_value = "/dev/spidev0.0")]
    spi_dev_path: String,

    #[structopt(default_value = "ipc:///tmp/gp2d12.ipc")]
    address: String,
}

fn main() {
    let opt = Opt::from_args();

    // Create nanomsg publisher socket
    let mut socket =
        Socket::new(Protocol::Pub).expect("could not create socket");
    socket
        .bind(opt.address.as_str())
        .expect("socket bind failed");

    // Instead of creating new messages modify an existing one
    let mut msg = PubMessage {
        pub_type: PubType::LongDistanceSensor,
        integral: 0,
        decimal: 0.0,
    };

    let sleep_duration = Duration::from_millis(opt.delay);

    if let Ok(mcp3008) = Mcp3008::new(&opt.spi_dev_path) {
        let mut gp2d12 = Gp2d12::new(mcp3008, opt.adc);

        let mut value_buffer = [0_f32; VALUE_BUFFER_SIZE];

        let mut write_index: usize = 0;
        let mut avg: f32 = 0.0;

        loop {
            match gp2d12.read() {
                Ok(value) => {
                    value_buffer[write_index] = value;
                    write_index = (write_index + 1) % VALUE_BUFFER_SIZE;
                }
                Err(err) => panic!(err),
            }

            let mut boxplot_array = value_buffer;
            quickersort::sort_floats(&mut boxplot_array[..]);

            for value in boxplot_array
                [EXCLUSION_RANGE..VALUE_BUFFER_SIZE - EXCLUSION_RANGE]
                .iter()
            {
                avg += value;
            }

            msg = fill_message_decimal(avg / LOGICAL_BOXPLOT_SIZE, msg);

            avg = 0.0;

            publish(&mut socket, &msg);

            sleep(sleep_duration);
        }
    } else {
        publish_random_values(
            socket,
            msg,
            sleep_duration,
            Range::new(0.0, 80.0),
        );
    }
}
