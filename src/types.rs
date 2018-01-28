use bincode::Bounded;
use bincode::serialize as bincode_serialize;
use bincode::deserialize as bincode_deserialize;

use std::mem::size_of;
use std::fmt::{self, Debug, Display};

lazy_static! {
    // There appears to by a 2 byte overhead when serializing to bincode
    static ref MSG_SIZE_LIMIT: Bounded = Bounded(size_of::<PubMessage>() as u64 + 2);
}

macro_rules! define_pub_types {
    ($Name:ident { $($Variant:ident),* $(,)* }) =>
    {
        #[derive(Serialize, Deserialize, PartialEq, Clone, Eq, Hash, Debug)]
        pub enum $Name {
            $($Variant),*,
        }
        pub const PUB_TYPES: &'static [$Name] = &[$($Name::$Variant),*];      
    }
}

define_pub_types!(PubType {
    PressureSensorTemperature,
    PressureSensorPressure,
    ShortDistanceSensor,
    LongDistanceSensor,
    AccelerometerX,
    AccelerometerY,
    AccelerometerZ,
    GyroscopeX,
    GyroscopeY,
    GyroscopeZ,
    PowerButton,
});

impl Display for PubType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}


#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct PubMessage {
    pub pub_type: PubType,
    pub value: i16,
}

pub fn str_to_pub_type(pub_type: &str) -> Option<PubType> {
    for pub_type_ in PUB_TYPES {
    	if pub_type_.to_string() == pub_type {
    		return Some(pub_type_.clone());
    	}
    }
    None
}

pub fn serialize(msg: &PubMessage) -> Option<Vec<u8>> {
    match bincode_serialize(&msg, *MSG_SIZE_LIMIT) {
        Ok(serialized_message) => Some(serialized_message),
        Err(_) => None,
    }
}

pub fn deserialize(msg: &[u8]) -> Option<PubMessage> {
    match bincode_deserialize(&msg.to_vec()) {
        Ok(pub_msg) => Some(pub_msg),
        Err(_) => None,
    }
}
