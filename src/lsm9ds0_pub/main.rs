#[macro_use]
extern crate lazy_static;

#[cfg(target_os = "linux")]
extern crate i2cdev;
extern crate nanomsg;
extern crate rand;
#[macro_use]
extern crate structopt;

extern crate shared;

use std::thread::sleep;
use std::time::Duration;

use std::f32::consts::PI;

use nanomsg::{Protocol, Socket};

#[cfg(target_os = "linux")]
use i2cdev::core::I2CDevice;
#[cfg(target_os = "linux")]
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

use rand::distributions::Range;

use structopt::StructOpt;

use shared::types::{PubMessage, PubType};
use shared::utils::{fill_message_decimal, fill_message_integral, publish,
                    publish_random_values};

lazy_static! {
    static ref BETWEEN: Range<f32> = Range::new(
        -180.0, 180.0);
}

const GRAVITY: f32 = 9.80665;

const ADDRESS_ACCELMAG: u16 = 0x1d;

const ACCELRANGE_2G: u8 = 0b000 << 3;
const ACCEL_MG_LSB_2G: f32 = 0.061;

const REGISTER_CTRL_REG1_XM: u8 = 0x20;
const REGISTER_CTRL_REG2_XM: u8 = 0x21;
const REGISTER_CTRL_REG5_XM: u8 = 0x24;

const REGISTER_OUT_X_L_A: u8 = 0x28;

#[derive(StructOpt, Debug)]
#[structopt(name = "lsm9ds0_pub")]
struct Opt {
    #[structopt(long = "i2c-dev-path", default_value = "/dev/i2c-1")]
    i2c_dev_path: String,

    #[structopt(default_value = "ipc:///tmp/lsm9ds0.ipc")]
    address: String,
}

struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}

#[cfg(target_os = "linux")]
struct EmulatedGyro {
    i2c: LinuxI2CDevice,
}

#[cfg(target_os = "linux")]
impl EmulatedGyro {
    fn new(i2c_dev_path: String) -> Result<EmulatedGyro, LinuxI2CError> {
        let mut i2c = LinuxI2CDevice::new(i2c_dev_path, ADDRESS_ACCELMAG)?;

        // Enable accelerometer continous
        i2c.smbus_write_byte_data(REGISTER_CTRL_REG1_XM, 0x67)?;
        i2c.smbus_write_byte_data(REGISTER_CTRL_REG5_XM, 0b11110000)?;

        let mut acc_reg = i2c.smbus_read_byte_data(REGISTER_CTRL_REG2_XM)?;
        acc_reg &= !(0b00111000);
        acc_reg |= ACCELRANGE_2G;
        i2c.smbus_write_byte_data(REGISTER_CTRL_REG2_XM, acc_reg)?;

        Ok(EmulatedGyro { i2c: i2c })
    }

    fn read_raw(&mut self, start_addr: u8) -> Result<[i16; 3], LinuxI2CError> {
        let mut values = [0_i16; 3];

        let mut cur_addr = start_addr;
        let mut index = 0;

        while cur_addr < start_addr + 6 {
            let low = self.i2c.smbus_read_byte_data(cur_addr)? as u16;
            let high = self.i2c.smbus_read_byte_data(cur_addr + 1)? as u16;

            let unsigned_val = (low | (high << 8)) as u32;

            values[index] = if unsigned_val > 32767 {
                (unsigned_val as i32 - 65536) as i16
            } else {
                unsigned_val as i16
            };

            cur_addr += 2;
            index += 1;
        }

        Ok(values)
    }

    fn read_acc(&mut self) -> Result<Vector3, LinuxI2CError> {
        match self.read_raw(REGISTER_OUT_X_L_A) {
            Ok(raw) => Ok(Vector3 {
                x: ((raw[0] as f32 * ACCEL_MG_LSB_2G) / 1000.0) * GRAVITY,
                y: ((raw[1] as f32 * ACCEL_MG_LSB_2G) / 1000.0) * GRAVITY,
                z: ((raw[2] as f32 * ACCEL_MG_LSB_2G) / 1000.0) * GRAVITY,
            }),
            Err(err) => Err(err),
        }
    }
}

fn main() {
    let opt = Opt::from_args();

    // Obtain config values
    let i2c_dev_path = opt.i2c_dev_path;

    // Create nanomsg publisher socket
    let mut socket =
        Socket::new(Protocol::Pub).expect("could not create socket");
    socket
        .bind(opt.address.as_str())
        .expect("socket bind failed");

    let sleep_duration = Duration::from_millis(10);

    #[cfg(target_os = "linux")]
    fn publish_values(
        mut socket: Socket,
        sleep_duration: Duration,
        i2c_dev_path: String,
    ) {
        let mut roll_msg = PubMessage {
            pub_type: PubType::GyroscopeX,
            integral: 0,
            decimal: 0.0,
        };

        let mut pitch_msg = PubMessage {
            pub_type: PubType::GyroscopeY,
            integral: 0,
            decimal: 0.0,
        };

        let mut is_falling_msg = PubMessage {
            pub_type: PubType::IsFalling,
            integral: 0,
            decimal: 0.0,
        };

        let mut accelerometer_z_msg = PubMessage {
            pub_type: PubType::AccelerometerZ,
            integral: 0,
            decimal: 0.0,
        };

        if let Ok(mut emulated_gyro) = EmulatedGyro::new(i2c_dev_path) {
            loop {
                let acc = emulated_gyro.read_acc().unwrap();

                //                    y
                //      roll = atan2(---)
                //                    z

                let mut roll = acc.y.atan2(acc.z);

                //                                 -x
                //      pitch = atan(-------------------------------)
                //                    y * sin(roll) + z * cos(roll)
                //
                let mut pitch = if acc.y * roll.sin() + acc.z * roll.cos()
                    == 0.0
                {
                    if acc.x > 0.0 {
                        PI / 2.0
                    } else {
                        -PI / 2.0
                    }
                } else {
                    (-acc.x / (acc.y * roll.sin() + acc.z * roll.cos())).atan()
                };

                roll = roll * 180.0 / PI;
                pitch = pitch * 180.0 / PI;

                // Calculations are inaccurate if falling
                if acc.z > 8.0 {
                    roll_msg = fill_message_decimal(roll, roll_msg);
                    pitch_msg = fill_message_decimal(pitch, pitch_msg);

                    publish(&mut socket, &roll_msg);
                    publish(&mut socket, &pitch_msg);
                }

                is_falling_msg = fill_message_integral(
                    if acc.z < 6.0 { 1 } else { 0 },
                    is_falling_msg,
                );

                publish(&mut socket, &is_falling_msg);

                accelerometer_z_msg =
                    fill_message_decimal(acc.z, accelerometer_z_msg);

                publish(&mut socket, &accelerometer_z_msg);

                sleep(sleep_duration);
            }
        } else {
            publish_random_values(socket, roll_msg, sleep_duration, *BETWEEN);
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn publish_values(
        socket: Socket,
        sleep_duration: Duration,
        _i2c_dev_path: String,
    ) {
        let msg = PubMessage {
            pub_type: PubType::GyroscopeX,
            integral: 0,
            decimal: 0.0,
        };

        publish_random_values(socket, msg, sleep_duration, *BETWEEN);
    }

    publish_values(socket, sleep_duration, i2c_dev_path);
}
