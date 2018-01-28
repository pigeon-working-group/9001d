#[macro_use]
extern crate lazy_static;

extern crate nanomsg;
extern crate rand;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

#[cfg(target_os = "linux")]
extern crate spidev;

extern crate shared;

use std::thread::sleep;
use std::time::Duration;

use nanomsg::{Protocol, Socket};

#[cfg(target_os = "linux")]
use spidev::{SPI_MODE_0, Spidev, SpidevOptions, SpidevTransfer};

use rand::distributions::Range;

use structopt::StructOpt;

use shared::utils::{publish, publish_random_values};
use shared::types::{str_to_pub_type, PubMessage};

lazy_static! {
    static ref BETWEEN: Range<i16> = Range::new(0, 1024);
}

#[derive(StructOpt, Debug)]
#[structopt(name = "mcp3008_pub")]
struct Opt {
    #[structopt(short = "a", long = "adc")]
    adc: u8,

    #[structopt(short = "t", long = "pub-type")]
    pub_type: String,

    #[structopt(long = "spi-dev-path", default_value = "/dev/spidev0.0")]
    spi_dev_path: String,

    #[structopt(default_value = "ipc:///tmp/gpio_1.ipc")]
    address: String,
}

#[cfg(target_os = "linux")]
struct Mcp3008 {
    spi: Spidev,
}

#[cfg(target_os = "linux")]
impl Mcp3008 {
    fn new(spi_dev_path: String) -> Option<Mcp3008> {
        let options = SpidevOptions::new()
            .max_speed_hz(1_000_000)
            .mode(SPI_MODE_0)
            .lsb_first(false)
            .build();

        if let Ok(mut spi) = Spidev::open(spi_dev_path) {
            if let Ok(_) = spi.configure(&options) {
                return Some(Mcp3008 { spi: spi });
            }
        }

        None
    }

    // https://github.com/adafruit/Adafruit_Python_GPIO/blob/master/Adafruit_GPIO/SPI.py
    fn read_adc(&mut self, adc_number: u8) -> Option<u16> {
        match adc_number {
            0...7 => {
                // Start bit, single channel read
                let mut command: u8 = 0b11 << 6;
                command |= (adc_number & 0x07) << 3;
                // Note the bottom 3 bits of command are 0, this is to account for the
                // extra clock to do the conversion, and the low null bit returned at
                // the start of the response.

                let tx_buf = [command, 0x0, 0x0];
                let mut rx_buf = [0_u8; 3];

                // Marked as own scope so that rx_buf isn't borrowed
                // anymore after the transfer() call
                {
                    let mut transfer = SpidevTransfer::read_write(&tx_buf, &mut rx_buf);

                    if let Err(_) = self.spi.transfer(&mut transfer) {
                        return None;
                    }
                }

                let mut result = (rx_buf[0] as u16 & 0x01) << 9;
                result |= (rx_buf[1] as u16 & 0xFF) << 1;
                result |= (rx_buf[2] as u16 & 0x80) >> 7;

                Some(result & 0x3FF)
            }
            _ => None,
        }
    }
}

fn main() {
    let opt = Opt::from_args();

    // Obtain config values
    let adc = opt.adc;
    let address = opt.address;
    let spi_dev_path = opt.spi_dev_path;
    let pub_type = str_to_pub_type(&opt.pub_type).expect("invalid publisher type");

    // Create nanomsg publisher socket
    let mut socket = Socket::new(Protocol::Pub).expect("could not create socket");
    socket.bind(address.as_str()).expect("socket bind failed");

    // Instead of creating new messages modify an existing one
    let msg = PubMessage {
        pub_type: pub_type,
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
        adc: u8,
        spi_dev_path: String,
    ) {
        if let Some(mut mcp3008) = Mcp3008::new(spi_dev_path) {
            loop {
                if let Some(value) = mcp3008.read_adc(adc) {
                    msg.value = value as i16;
                } else {
                    panic!("could not read from Mcp3008")
                }

                publish(&mut socket, &msg);

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
        _adc: u8,
        _spi_dev_path: String,
    ) {
        publish_random_values(socket, msg, sleep_duration, *BETWEEN);
    }

    publish_values(socket, msg, sleep_duration, adc, spi_dev_path);
}
