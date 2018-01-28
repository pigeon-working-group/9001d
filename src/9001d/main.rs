extern crate bodyparser;
extern crate config;
extern crate iron;
extern crate mount;
extern crate persistent;
extern crate staticfile;

extern crate nanomsg;

#[macro_use]
extern crate quick_error;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate shared;

use std::path::Path;

use std::sync::Arc;
use std::thread;

use std::io::{Read, Write};

use iron::prelude::*;
use iron::status;
use iron::typemap::Key;
use mount::Mount;
use staticfile::Static;
use persistent::State;

use nanomsg::{Protocol, Socket};
use nanomsg::Error as NanomsgError;

use shared::utils::get_config;
use shared::types::{deserialize, PubMessage, PUB_TYPES};

quick_error! {
    #[derive(Debug)]
    pub enum PigeonError {
        HoverPositionUnreachable(position: f32) {
            display("height {} unreachable.", position)
        }
    }
}

struct Pigeon {
    position: f32,
    power: bool,
}


impl Key for Pigeon { type Value = Pigeon; }

impl Pigeon {
    fn new() -> Pigeon {
        Pigeon {
            position: 0.0,
            power: false,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Hover {
    position: f32,
}

#[derive(Deserialize, Debug, Clone)]
struct Power {
    power: bool,
}


fn position(req: &mut Request) -> IronResult<Response> {
	let pigeon_arc = req.get::<State<Pigeon>>().unwrap();
	let pigeon_r = pigeon_arc.read().unwrap();

	let req_body = req.get::<bodyparser::Struct<Hover>>();
	match req_body {
		Ok(Some(req_body)) => println!("Parsed body:\n{:?}", req_body),
		Ok(None) => println!("No body"),
		Err(err) => println!("Error: {:?}", err)
	}	
	Ok(Response::with((status::Ok, pigeon_r.position.to_string())))
}

fn publisher_types(_req: &mut Request) -> IronResult<Response> {
    match serde_json::to_string(&PUB_TYPES) {
        Ok(pub_types) => {
            Ok(Response::with((status::Ok, pub_types)))
        }
        Err(_) => {
            Ok(Response::with(status::InternalServerError))
        }
    }
}

fn main() {
    let config = get_config("9001d").expect("could not create config");

    let address = config.get_str("address").unwrap();

    let pigeon = Pigeon::new();

    // Receives messages from producers
    let mut sub_socket = Socket::new(Protocol::Sub).unwrap();
    // Subscribe to every topic
    sub_socket.subscribe("".as_bytes()).ok();
    // Forwards messages to WebSocket clients
    let mut pub_socket = Socket::new(Protocol::Pub).unwrap();
    pub_socket
        .bind(address.as_str())
        .expect("socket bind failed");


    // Subscribe to publishers defined in config
    for publisher in config.get_array("publishers").unwrap_or(vec![]) {
        if let Ok(publisher) = publisher.into_str() {
            match sub_socket.connect(&publisher) {
                Ok(_) => {
                    println!("connected to '{}'", &publisher);
                }
                Err(err) => {
                    panic!(err);
                }
            }
        }
    }

    thread::spawn(move || {
        // Fixed buffer length 
        let mut raw_msg = [0u8; 8];
        let mut pub_msg: PubMessage;

        loop {
            if let Err(err) = sub_socket.read(&mut raw_msg) {
                panic!(err);
            } else {
                pub_msg = deserialize(&raw_msg).unwrap();
                println!("{:?}", pub_msg);
                // Non-blocking message forwarding to WebSocket clients
                // Failure is ignored,
                pub_socket.nb_write(&raw_msg).ok();                
            }
        }  
    });

	let mut mount = Mount::new();
	
    let mut position_chain = Chain::new(position);
	position_chain.link(State::<Pigeon>::both(pigeon));

    mount.mount("/", Static::new(Path::new("web/9001-mission_control/index.html")));
    mount.mount("/static/", Static::new(Path::new("web/9001-mission_control/bower_components")));

	mount.mount("/position", position_chain);
    mount.mount("/publisher-types", publisher_types);
  

	Iron::new(mount).http("0.0.0.0:3000").unwrap();
}
