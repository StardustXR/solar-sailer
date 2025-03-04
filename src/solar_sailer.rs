use std::f32::consts::FRAC_PI_2;

use glam::{vec3, Affine3A, Mat4, Quat, Vec3};
use stardust_xr_fusion::{
	drawable::{Line, Lines, LinesAspect as _},
	input::{InputData, InputDataType},
	spatial::{SpatialRef, Transform},
	values::color::{color_space::LinearRgb, rgba_linear, Rgba},
};
use stardust_xr_molecules::lines::{circle, LineExt};

use crate::{input::Input, monado_movement::MonadoMovement, zone_movement::ZoneMovement};

pub struct SolarSailer {
	pub monado_movement: Option<MonadoMovement>,
	pub signifiers: Lines,
	pub mode: Mode,
	pub input: Input,
	pub grab_color: Rgba<f32, LinearRgb>,
	pub zone_movement: ZoneMovement,
	pub hmd: SpatialRef,
}

impl SolarSailer {
	pub fn handle_events(&mut self) {
		self.zone_movement.update_zone();
	}

	pub fn handle_input(&mut self) {
		self.input.handle_input();
	}
	pub async fn apply_offset(&mut self, delta_secs: f32) {
		match (&self.mode, self.monado_movement.as_mut()) {
			(Mode::MonadoOffset, Some(monado)) => monado.apply_offset(delta_secs, &self.hmd).await,
			(Mode::Zone, _) => self.zone_movement.apply_offset(delta_secs, &self.hmd).await,
			_ => {}
		}
	}

	pub fn update_velocity(&mut self, delta_secs: f32) {
		let offset = self.input.waft(delta_secs);
		match self.mode {
			Mode::Zone => {
				self.zone_movement.velocity *= 0.99;
				self.zone_movement.velocity += offset
			}
			Mode::MonadoOffset => {
				if let Some(monado_movement) = self.monado_movement.as_mut() {
					monado_movement.velocity *= 0.99;
					monado_movement.velocity += offset;
				}
			}
			Mode::Disabled => {}
		}
	}
	pub fn update_signifiers(&self) {
		if matches!(self.mode, Mode::Disabled) {
			self.signifiers.set_lines(&[]).unwrap();
			return;
		}
		let signifier_lines = self
			.input
			.update_signifiers(|input, grabbing| self.generate_signifier(input, grabbing));
		self.signifiers.set_lines(&signifier_lines).unwrap();
	}
	fn generate_signifier(&self, input: &InputData, grabbing: bool) -> Line {
		let transform = match &input.input {
			InputDataType::Pointer(_) => panic!("awawawawawawa"),
			InputDataType::Hand(h) => {
				Mat4::from_rotation_translation(h.palm.rotation.into(), h.palm.position.into())
					* Mat4::from_translation(vec3(0.0, 0.05, -0.02))
					* Mat4::from_rotation_x(FRAC_PI_2)
			}
			InputDataType::Tip(t) => {
				Mat4::from_rotation_translation(t.orientation.into(), t.origin.into())
			}
		};

		let line = circle(
			64,
			0.0,
			match &input.input {
				InputDataType::Pointer(_) => panic!("awawawawawawa"),
				InputDataType::Hand(_) => 0.1,
				InputDataType::Tip(_) => 0.0025,
			},
		)
		.transform(transform);
		if grabbing {
			line.color(self.grab_color)
		} else if matches!(self.mode, Mode::MonadoOffset) {
			line.color(rgba_linear!(1.0, 1.0, 0.0, 1.0))
		} else {
			line
		}
	}
}

#[derive(Debug)]
pub enum Mode {
	Zone,
	MonadoOffset,
	Disabled,
}

pub fn mat_from_transform(transform: &Transform) -> Affine3A {
	Affine3A::from_scale_rotation_translation(
		transform.scale.map(Vec3::from).unwrap_or(Vec3::ONE),
		transform.rotation.map(Quat::from).unwrap_or(Quat::IDENTITY),
		transform.translation.map(Vec3::from).unwrap_or(Vec3::ZERO),
	)
}


 
