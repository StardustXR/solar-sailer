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
		self.input.update_signifiers(self.mode);
	}
}

#[derive(Debug,Clone, Copy)]
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


 
