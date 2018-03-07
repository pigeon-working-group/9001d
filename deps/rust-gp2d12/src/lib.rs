extern crate mcp3008;

use mcp3008::{Mcp3008, Mcp3008Error};

const A_F1: f32 = 26.0 / 44625.0;
const B_F1: f32 = -256.0 / 525.0;
const C_F1: f32 = 14284.0 / 119.0;

const A_F2: f32 = 17.0 / 316848.0;
const B_F2: f32 = -14761.0 / 158424.0;
const C_F2: f32 = 307535.0 / 6601.0;

pub struct Gp2d12 {
	adc_number: u8,
	mcp3008: Mcp3008,
}

impl Gp2d12 {
	pub fn new(mcp3008: Mcp3008, adc_number: u8) -> Gp2d12 {
		Gp2d12 {
			adc_number: adc_number,
			mcp3008: mcp3008,
		}
	}

	pub fn read(&mut self) -> Result<f32, Mcp3008Error> {
		let raw_value = self.mcp3008.read_adc(self.adc_number)? as u32;
		let raw_value_f32 = raw_value as f32;

		Ok(if raw_value < 360 {
			raw_value.pow(2) as f32 * A_F1 + raw_value_f32 * B_F1 + C_F1
		} else {
			raw_value.pow(2) as f32 * A_F2 + raw_value_f32 * B_F2 + C_F2
		})
	}
}
