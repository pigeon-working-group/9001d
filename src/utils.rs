use std::io::Write;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use clap::{App, Arg};
use config::{Config, ConfigError, File};
use nanomsg::Socket;

use rand::distributions::{IndependentSample, Range};
use rand::thread_rng;

use colored::*;

use types::{serialize, PubMessage};

pub fn publish_random_values(
    mut socket: Socket,
    mut msg: PubMessage,
    sleep_duration: Duration,
    between: Range<f32>,
) {
    println!("{}", "Publishing random values".yellow());

    let mut rng = thread_rng();
    loop {
        msg = fill_message_decimal(between.ind_sample(&mut rng), msg);

        if let Err(err) = socket.write_all(&serialize(&msg).unwrap()[..]) {
            panic!(err);
        }
        sleep(sleep_duration);
    }
}

pub fn publish(socket: &mut Socket, msg: &PubMessage) {
    if let Err(err) = socket.write_all(&serialize(msg).unwrap()[..]) {
        panic!(err);
    }
}

pub fn fill_message_decimal(decimal: f32, mut msg: PubMessage) -> PubMessage {
    msg.decimal = decimal;

    msg.integral = decimal as i16;

    msg
}

pub fn fill_message_integral(integral: i16, mut msg: PubMessage) -> PubMessage {
    msg.integral = integral;

    msg.decimal = msg.integral as f32;

    msg
}

pub fn config_stem(path: &str) -> Option<&str> {
    let path = Path::new(path);
    let ext = path.extension().map(|s| s.to_str())??;

    match path.is_file() && ["toml", "json", "yaml", "hjson"].contains(&ext) {
        true => Some(path_without_extension(path)?),
        false => None,
    }
}

pub fn path_without_extension(path: &Path) -> Option<&str> {
    let ext = path.extension().map(|s| s.to_str())??;

    path.to_str().map(|s| &s[..s.len() - ext.len()])
}

pub fn get_config(app_name: &str) -> Result<Config, ConfigError> {
    let mut config = Config::default();

    let matches = App::new(app_name)
        .author("Philip Trauner <philip.trauner@arztpraxis.io>")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets config file")
                .required(false)
                .takes_value(true),
        )
        .get_matches();

    let config_file;

    match matches.value_of("config") {
        Some(config_path) => match config_stem(config_path) {
            Some(stem) => config_file = File::with_name(stem),
            None => {
                return Err(ConfigError::Message(String::from(
                    "could not load from specified config path",
                )));
            }
        },
        None => {
            config_file = File::with_name(app_name);
        }
    }

    config.merge(config_file)?;

    Ok(config)
}
