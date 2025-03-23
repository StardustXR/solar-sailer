use std::sync::Arc;

use glam::{Quat, Vec3};
use libmonado::{Monado, Pose};
use stardust_xr_fusion::{
	objects::play_space,
	spatial::{SpatialRef, SpatialRefAspect},
	ClientHandle,
};

use crate::solar_sailer::mat_from_transform;

pub struct MonadoMovement {
	monado: Monado,
	pub velocity: Vec3,
	stage: SpatialRef,
}

impl MonadoMovement {
	pub async fn apply_offset(&mut self, delta_secs: f32, velocity_ref: &SpatialRef) {
		let origins = self
			.monado
			.tracking_origins()
			.unwrap()
			.into_iter()
			.collect::<Vec<_>>();

		let Some(Pose {
			position,
			orientation,
		}) = origins.first().and_then(|o| o.get_offset().ok())
		else {
			return;
		};

		if self.velocity.length_squared() > 0.001 {
			let Ok(transform) = velocity_ref.get_transform(&self.stage).await else {
				return;
			};
			let mat = mat_from_transform(&transform);
			let delta_position = mat.transform_vector3(self.velocity * -1.0 * delta_secs);
			let offset_position = Vec3::from(position) + (delta_position);

			for origin in origins.iter() {
				let _ = origin.set_offset(Pose {
					position: offset_position.into(),
					orientation,
				});
			}
		}
	}
	pub async fn from_monado(client: &Arc<ClientHandle>, monado: Option<Monado>) -> Option<Self> {
		let monado = monado?;
		for origin in monado.tracking_origins().unwrap().into_iter() {
			let _ = origin.set_offset(Pose {
				position: Vec3::ZERO.into(),
				orientation: Quat::IDENTITY.into(),
			});
		}
		Some(MonadoMovement {
			monado,
			velocity: Vec3::ZERO,
			stage: play_space(client).await?.spatial,
		})
	}
}
