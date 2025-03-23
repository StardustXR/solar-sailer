use std::{collections::HashMap, sync::Arc};

use glam::Vec3;
use stardust_xr_fusion::{
	ClientHandle,
	fields::{Field, Shape},
	node::{NodeResult, NodeType},
	spatial::{
		Spatial, SpatialAspect, SpatialRef, SpatialRefAspect, Transform, Zone, ZoneAspect,
		ZoneEvent,
	},
};

use crate::solar_sailer::mat_from_transform;

pub struct ZoneMovement {
	pub velocity: Vec3,
	offset: Vec3,
	zone: Zone,
	_zone_field: Field,
	zone_spatial: Spatial,
	captured: HashMap<u64, Spatial>,
}

impl ZoneMovement {
	pub fn update_zone(&mut self) {
		_ = self.zone.update();
		while let Some(event) = self.zone.recv_zone_event() {
			match event {
				ZoneEvent::Capture { spatial } => {
					println!("capturing spatial");
					_ = spatial.set_spatial_parent_in_place(&self.zone);
					self.captured.insert(spatial.id(), spatial);
				}
				ZoneEvent::Enter { spatial } => {
					println!("spatial entered");
					_ = self.zone.capture(&spatial);
				}
				ZoneEvent::Release { id } => {
					println!("releasing spatial");
					self.captured.remove(&id);
				}
				_ => {}
			}
		}
	}
	pub async fn apply_offset(&mut self, delta_secs: f32, velocity_ref: &SpatialRef) {
		let mat = mat_from_transform(
			&velocity_ref
				.get_transform(&self.zone_spatial)
				.await
				.unwrap(),
		);
		self.offset += mat.transform_vector3(self.velocity * delta_secs);
		_ = self
			.zone_spatial
			.set_local_transform(Transform::from_translation(self.offset));
	}

	pub fn new(client: &Arc<ClientHandle>) -> NodeResult<Self> {
		let zone_spatial = Spatial::create(client.get_root(), Transform::identity(), false)?;
		let zone_field = Field::create(
			&zone_spatial,
			Transform::identity(),
			Shape::Sphere(1000.0), 
		)?;
		let zone = Zone::create(&zone_spatial, Transform::identity(), &zone_field)?;
		Ok(ZoneMovement {
			velocity: Vec3::ZERO,
			offset: Vec3::ZERO,
			zone,
			_zone_field: zone_field,
			zone_spatial,
			captured: HashMap::new(),
		})
	}
}
