#[macro_use]
extern crate lazy_static;

#[cfg(target_os = "linux")]
extern crate i2cdev;
extern crate nanomsg;
extern crate rand;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

extern crate shared;

use std::thread::sleep;
use std::time::Duration;
use std::collections::HashMap;

use nanomsg::{Protocol, Socket};

#[cfg(target_os = "linux")]
use i2cdev::linux::LinuxI2CDevice;
#[cfg(target_os = "linux")]
use i2cdev::core::I2CDevice;

use rand::distributions::Range;

use structopt::StructOpt;

use shared::utils::{publish, publish_random_values};
use shared::types::{PubMessage, PubType};

// REGISTERS is only used on Linux
// Compiling on other platforms would otherwise result in
// unused_variables errors
lazy_static! {
    #[allow(unused_variables)]
    static ref REGISTERS: HashMap<PubType, (u8, u8)> = {
        [
            (PubType::GyroscopeX, (0x28, 0x29)),
            (PubType::GyroscopeY, (0x2A, 0x2B)),
            (PubType::GyroscopeZ, (0x2C, 0x2D))
        ].iter().cloned().collect()
    };

    static ref BETWEEN: Range<i16> = Range::new(
        i16::min_value(), i16::max_value());
}

#[derive(StructOpt, Debug)]
#[structopt(name = "lsm9ds0_pub")]
struct Opt {
    #[structopt(long = "i2c-dev-path", default_value = "/dev/i2c-1")]
    i2c_dev_path: String,

    #[structopt(default_value = "ipc:///tmp/lsm9ds0.ipc")]
    address: String,
}

fn main() {
    let opt = Opt::from_args();

    // Obtain config values
    let address = opt.address;
    let i2c_dev_path = opt.i2c_dev_path;

    // Create nanomsg publisher socket
    let mut socket = Socket::new(Protocol::Pub).expect("could not create socket");
    socket.bind(address.as_str()).expect("socket bind failed");

    // Instead of creating new message modify existing one
    let msg = PubMessage {
        pub_type: PubType::GyroscopeX,
        value: 0,
    };

    // std::thread::yield_now() can not be used to prevent excessive CPU
    // usage -> sleep 1ms instead
    let sleep_duration = Duration::from_millis(1);

    #[cfg(target_os = "linux")]
    fn publish_values(
        mut socket: Socket,
        mut msg: PubMessage,
        sleep_duration: Duration,
        i2c_dev_path: String,
    ) {
        if let Ok(mut i2c) = LinuxI2CDevice::new(i2c_dev_path, 0x6b) {
            // Device initialization
            i2c.smbus_write_byte_data(0x20, 0b00001111)
                .expect("could not initialize lsm9ds0");

            loop {
                for (pub_type, registers) in REGISTERS.iter() {
                    let val1 = i2c.smbus_read_byte_data((*registers).0).unwrap() as u16;
                    let val2 = i2c.smbus_read_byte_data((*registers).1).unwrap() as u16;

                    let raw_val = val2 * 256 + val1;

                    // Offsets can not be defined as constants yet
                    if raw_val > i16::max_value() as u16 {
                        msg.value = (raw_val as i32 - u16::max_value() as i32) as i16;
                    } else {
                        msg.value = raw_val as i16;
                    }

                    msg.pub_type = pub_type.clone();

                    publish(&mut socket, &msg);
                }

                sleep(sleep_duration);
            }
        } else {
            publish_random_values(socket, msg, sleep_duration, *BETWEEN);
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn publish_values(
        socket: Socket,
        msg: PubMessage,
        sleep_duration: Duration,
        _i2c_dev_path: String,
    ) {
        publish_random_values(socket, msg, sleep_duration, *BETWEEN);
    }

    publish_values(socket, msg, sleep_duration, i2c_dev_path);
}
