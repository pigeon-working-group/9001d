extern crate nanomsg;
extern crate rand;
extern crate rppal;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

extern crate shared;

use std::thread::sleep;
use std::time::Duration;

use nanomsg::{Protocol, Socket};

use rppal::gpio::{Gpio, Level, Mode};
use rppal::system::DeviceInfo;

use rand::distributions::Range;

use shared::utils::{publish, publish_random_values};
use shared::types::{str_to_pub_type, PubMessage};

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "gpio_pub")]
struct Opt {
    #[structopt(short = "p", long = "pin")]
    pin: u8,

    #[structopt(short = "t", long = "pub-type")]
    pub_type: String,

    #[structopt(default_value = "ipc:///tmp/gpio_1.ipc")]
    address: String,
}

fn main() {
    let opt = Opt::from_args();

    // Obtain config values
    let pin = opt.pin;
    let address = opt.address;
    let pub_type = str_to_pub_type(&opt.pub_type).expect("invalid publisher type");

    // Create nanomsg publisher socket
    let mut socket = Socket::new(Protocol::Pub).unwrap();
    socket.bind(address.as_str()).expect("socket bind failed");

    // Instead of creating new messages modify an existing one
    let mut msg = PubMessage {
        pub_type: pub_type,
        value: 0,
    };

    // std::thread::yield_now() can not be used to prevent excessive CPU
    // usage -> sleep 1ms instead
    let sleep_duration = Duration::from_millis(1);

    // Generate random values if not running on Raspberry Pi
    if DeviceInfo::new().is_err() {
        publish_random_values(socket, msg, sleep_duration, Range::new(0, 2));
    } else {
        let mut gpio = Gpio::new().expect("could not create Gpio instance");
        gpio.set_mode(pin, Mode::Input);

        loop {
            if let Ok(level) = gpio.read(pin) {
                msg.value = match level {
                    Level::High => 1,
                    Level::Low => 0,
                }
            } else {
                panic!("could not read from Gpio pin")
            }

            publish(&mut socket, &msg);

            sleep(sleep_duration);
        }
    }
}
