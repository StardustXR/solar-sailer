use glam::{Affine3A, Quat, Vec3};
use stardust_xr_fusion::{
	spatial::Transform,
	values::color::{Rgba, color_space::LinearRgb},
};

use crate::{input::Input, monado_movement::MonadoMovement, zone_movement::ZoneMovement};

pub struct SolarSailer {
	pub monado_movement: Option<MonadoMovement>,
	pub mode: Mode,
	pub input: Input,
	pub zone_movement: ZoneMovement,
}

impl SolarSailer {
	pub fn handle_events(&mut self) {
		self.zone_movement.update_zone();
	}

	pub fn handle_input(&mut self) {
		self.input.handle_input();
	}
	pub async fn apply_offset(&mut self, delta_secs: f32) {
		let vel_ref = &self.input.get_velocity_space();
		match (&self.mode, self.monado_movement.as_mut()) {
			(Mode::MonadoOffset, Some(monado)) => monado.apply_offset(delta_secs, vel_ref).await,
			(Mode::Zone, _) => self.zone_movement.apply_offset(delta_secs, vel_ref).await,
			_ => {}
		}
	}

	pub async fn update_velocity(&mut self, delta_secs: f32) {
		let offset = self.input.waft(delta_secs).await;
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

#[derive(Debug, Clone, Copy)]
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
