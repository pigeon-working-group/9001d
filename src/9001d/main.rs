extern crate config;

extern crate iron;
extern crate mount;
extern crate staticfile;

extern crate nanomsg;

extern crate rppal;

extern crate serde_json;

extern crate colored;

extern crate shared;

use std::path::Path;

use std::time::{Duration, Instant};

use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use std::collections::HashMap;
use std::vec::Vec;

use std::io::Read;

use iron::prelude::*;
use iron::status;
use mount::Mount;
use staticfile::Static;

use rppal::gpio::{Gpio, Level, Mode};

use colored::*;

use nanomsg::{Protocol, Socket};
use nanomsg::Error as NanomsgError;

use shared::types::{deserialize, PubMessage, PubType, PUB_TYPES};
use shared::utils::get_config;

const NANOSEC_TO_MILLISEC: u32 = 1000000;

fn publisher_types(_req: &mut Request) -> IronResult<Response> {
    Ok(match serde_json::to_string(&PUB_TYPES) {
        Ok(pub_types) => Response::with((status::Ok, pub_types)),
        Err(_) => Response::with(status::InternalServerError),
    })
}

type MessageCache = HashMap<PubType, (i16, f32)>;

struct WrappedMessageCache {
    message_cache: MessageCache,
}

// Populates message cache
impl WrappedMessageCache {
    fn new() -> WrappedMessageCache {
        let mut message_cache = HashMap::new();

        // Pre-populate message cache with all possible message types
        for pub_type in PUB_TYPES {
            message_cache.insert(pub_type.clone(), (0, 0.0));
        }

        WrappedMessageCache {
            message_cache: message_cache,
        }
    }

    fn clone_inner(&mut self) -> MessageCache {
        self.message_cache.clone()
    }
}

struct Pigeon {
    exp_deceleration: f32,
    tolerance: f32,
    target_altitude: f32,
}

fn halt_altitude(altitude: f32, velocity: f32, deceleration: f32) -> f32 {
    altitude - ((velocity.abs() + 68.649).powf(2.0) / (2.0 * deceleration))
}

fn is_above(alt_above: f32, alt_below: f32, tolerance: f32) -> bool {
    alt_above > (alt_below + tolerance)
}

impl Pigeon {
    fn new(
        exp_deceleration: f32,
        tolerance: f32,
        target_altitude: f32,
    ) -> Pigeon {
        Pigeon {
            exp_deceleration: exp_deceleration,
            tolerance: tolerance,
            target_altitude: target_altitude,
        }
    }

    fn control_loop(
        &mut self,
        wrapped_message_cache_arc: Arc<Mutex<WrappedMessageCache>>,
    ) {
        let mut msg_cache: MessageCache;

        let max_loop_duration = Duration::from_millis(10);
        let mut loop_time: Instant;

        let mut fall_time = Instant::now();
        let mut fall_time_set = false;

        let mut gpio = Gpio::new().unwrap();
        gpio.set_mode(20, Mode::Output);
        gpio.set_mode(21, Mode::Output);

        /*
        let operation_ratio_arc_1 = Arc::new(Mutex::new(0.0));
        let operation_ratio_arc_2 = Arc::new(Mutex::new(0.0));

        let operation_ratio_arc_1_clone = operation_ratio_arc_1.clone();
        let operation_ratio_arc_2_clone = operation_ratio_arc_2.clone();

        let cycle_time = Duration::from_millis(150);
        */

        /*
        thread::spawn(move || {
            Pigeon::valve_controller(
                operation_ratio_arc_1_clone,
                &cycle_time,
                20,
            );
        });

        thread::spawn(move || {
            Pigeon::valve_controller(
                operation_ratio_arc_2_clone,
                &cycle_time,
                21,
            );
        });
        */

        let mut is_boosting = false;

        msg_cache = wrapped_message_cache_arc.lock().unwrap().clone_inner();

        let mut curr_velocity = 0.0;

        let mut last_altitude = msg_cache[&PubType::LongDistanceSensor].1;

        thread::sleep(max_loop_duration);

        loop {
            loop_time = Instant::now();

            msg_cache = wrapped_message_cache_arc.lock().unwrap().clone_inner();

            curr_velocity =
                if msg_cache[&PubType::LongDistanceSensor].1 == last_altitude {
                    curr_velocity
                } else {
                    // cm / s
                    (msg_cache[&PubType::LongDistanceSensor].1 - last_altitude)
                        * 100.0
                };

            if is_above(
                msg_cache[&PubType::LongDistanceSensor].1,
                self.target_altitude,
                self.tolerance + 5.0,
            ) {
                if msg_cache[&PubType::IsFalling].0 == 1 {
                    if !fall_time_set {
                        fall_time = Instant::now();
                        fall_time_set = true;
                    }

                    if !is_above(
                        halt_altitude(
                            msg_cache[&PubType::LongDistanceSensor].1,
                            curr_velocity,
                            self.exp_deceleration,
                        ),
                        self.target_altitude,
                        self.tolerance + 2.0,
                    ) {
                        gpio.write(20, Level::High);
                        gpio.write(21, Level::High);

                        is_boosting = true;
                    }
                }
            } else {
                gpio.write(20, Level::Low);
                gpio.write(21, Level::Low);

                fall_time_set = false;
                is_boosting = false;
            }

            if fall_time_set {
                println!(
                    "is_boosting: {} altitude: {:>8} vel: {} \
                     current_fall_duration: {}",
                    is_boosting as u8,
                    msg_cache[&PubType::LongDistanceSensor].1,
                    curr_velocity,
                    fall_time.elapsed().subsec_nanos() / NANOSEC_TO_MILLISEC,
                );
            }

            last_altitude = msg_cache[&PubType::LongDistanceSensor].1;

            if loop_time.elapsed() > max_loop_duration {
                println!("{}", "Maximum loop time exceeded".red());
            } else {
                thread::sleep(max_loop_duration - loop_time.elapsed());
            }
        }
    }

    fn valve_controller(
        operation_ratio_arc: Arc<Mutex<f32>>,
        cycle_time: &Duration,
        pin: u8,
    ) {
        let mut operation_ratio: f32;

        if let Ok(mut gpio) = Gpio::new() {
            gpio.set_mode(pin, Mode::Output);

            loop {
                operation_ratio = *operation_ratio_arc.lock().unwrap();

                gpio.write(pin, Level::High);

                thread::sleep(Duration::from_millis(
                    ((cycle_time.subsec_nanos() / NANOSEC_TO_MILLISEC) as f32
                        * operation_ratio) as u64,
                ));
                gpio.write(pin, Level::Low);

                thread::sleep(Duration::from_millis(
                    ((cycle_time.subsec_nanos() / NANOSEC_TO_MILLISEC) as f32
                        * (1.0 - operation_ratio)) as u64,
                ));
            }
        }
    }
}

struct Consumer {
    sub_socket: Socket,
    pub_socket: Socket,
}

impl Consumer {
    fn new(
        publishers: Vec<String>,
        pub_address: &str,
    ) -> Result<Consumer, NanomsgError> {
        let mut sub_socket = Socket::new(Protocol::Sub)?;

        sub_socket.subscribe(b"").ok();

        for publisher in publishers {
            sub_socket.connect(&publisher).expect(&format!(
                "connection to publisher failed ({})",
                publisher
            ));
        }

        // Used to forward messages to 9001-mission_control
        let mut pub_socket = Socket::new(Protocol::Pub)?;
        pub_socket.bind(pub_address)?;

        Ok(Consumer {
            sub_socket: sub_socket,
            pub_socket: pub_socket,
        })
    }

    fn consume(
        &mut self,
        wrapped_message_cache_arc: Arc<Mutex<WrappedMessageCache>>,
    ) {
        let mut raw_msg = [0u8; 16];
        let mut pub_msg: PubMessage;

        loop {
            if let Err(err) = self.sub_socket.read(&mut raw_msg) {
                panic!(err);
            } else {
                pub_msg = deserialize(&raw_msg).unwrap();

                if let Ok(mut wrapped_message_cache) =
                    wrapped_message_cache_arc.lock()
                {
                    wrapped_message_cache.message_cache.insert(
                        pub_msg.pub_type,
                        (pub_msg.integral, pub_msg.decimal),
                    );
                }

                // Non-blocking message forwarding to WebSocket clients
                // Failure is ignored,
                self.pub_socket.nb_write(&raw_msg).ok();
            }
        }
    }
}

fn main() {
    let config = get_config("9001d").expect("could not create config");

    let address = config.get_str("address").unwrap();

    let mut publishers: Vec<String> = Vec::new();

    for publisher in config.get_array("publishers").unwrap_or(vec![]) {
        if let Ok(publisher) = publisher.into_str() {
            publishers.push(publisher);
        }
    }

    let wrapped_msg_cache_arc =
        Arc::new(Mutex::new(WrappedMessageCache::new()));

    let wrapped_msg_cache_arc_consumer = wrapped_msg_cache_arc.clone();
    let wrapped_msg_cache_arc_pigeon = wrapped_msg_cache_arc.clone();

    let mut consumer = Consumer::new(publishers, &address).unwrap();

    thread::spawn(move || {
        consumer.consume(wrapped_msg_cache_arc_consumer);
    });

    let exp_deceleration =
        config.get_float("exp_deceleration").unwrap_or(100.0) as f32;
    let tolerance = config.get_float("tolerance").unwrap_or(5.0) as f32;
    let target_altitude =
        config.get_float("target_altitude").unwrap_or(8.0) as f32;

    let mut pigeon = Pigeon::new(exp_deceleration, tolerance, target_altitude);

    thread::spawn(move || {
        pigeon.control_loop(wrapped_msg_cache_arc_pigeon);
    });

    let mut mount = Mount::new();

    mount.mount(
        "/",
        Static::new(Path::new("web/9001-mission_control/index.html")),
    );
    mount.mount(
        "/static/",
        Static::new(Path::new("web/9001-mission_control/node_modules")),
    );

    mount.mount("/publisher-types", publisher_types);

    Iron::new(mount).http("0.0.0.0:3000").unwrap();
}
