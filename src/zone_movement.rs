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
use tokio::task::JoinSet;

use crate::solar_sailer::mat_from_transform;

pub struct ZoneMovement {
	pub velocity: Vec3,
	zone: Zone,
	_zone_field: Field,
	zone_spatial: Spatial,
	captured: HashMap<u64, Spatial>,
	entered: HashMap<u64, SpatialRef>,
}

impl ZoneMovement {
	pub fn update_zone(&mut self) {
		_ = self.zone.update();
		while let Some(event) = self.zone.recv_zone_event() {
			match event {
				ZoneEvent::Capture { spatial } => {
					println!("capturing spatial");
					self.captured.insert(spatial.id(), spatial);
				}
				ZoneEvent::Enter { spatial } => {
					println!("spatial entered");
					self.entered.insert(spatial.id(), spatial);
				}
				ZoneEvent::Release { id } => {
					println!("releasing spatial");
					self.captured.remove(&id);
				}
				ZoneEvent::Leave { id } => {
					println!("spatial left");
					self.entered.remove(&id);
				}
			}
		}
	}
	pub async fn apply_offset(&mut self, delta_secs: f32, velocity_ref: &SpatialRef) {
		if self.velocity.length_squared() < f32::EPSILON * 10.0 {
			self.velocity = Vec3::ZERO
		}
		match self.velocity == Vec3::ZERO {
			true => {
				for spatial in self
					.entered
					.iter()
					.filter(|(id, _)| !self.captured.contains_key(id))
					.map(|(_, s)| s)
				{
					if let Err(err) = self.zone.capture(spatial) {
						println!("unable to capture spatial: {err}");
					}
				}
			}
			false => {
				for spatial in self.captured.values() {
					if let Err(err) = self.zone.release(spatial) {
						println!("unable to release spatial: {err}");
					}
				}
			}
		}
		let mat = mat_from_transform(
			&velocity_ref
				.get_transform(&self.zone_spatial)
				.await
				.unwrap(),
		);
		let movement = mat.transform_vector3(self.velocity * delta_secs);
		let mut set = JoinSet::new();
		for v in self.captured.values() {
			let v = v.clone();
			let spatial = self.zone_spatial.clone();
			set.spawn(async move {
				if let Ok(mut transform) = v.get_transform(&spatial).await {
					let vec = transform
						.translation
						.get_or_insert_with(|| Vec3::ZERO.into());
					vec.x += movement.x;
					vec.y += movement.y;
					vec.z += movement.z;
					_ = v.set_relative_transform(&spatial, transform);
				}
			});
		}
		set.join_all().await;
	}

	pub fn new(client: &Arc<ClientHandle>) -> NodeResult<Self> {
		let zone_spatial = Spatial::create(client.get_root(), Transform::identity(), false)?;
		let zone_field =
			Field::create(&zone_spatial, Transform::identity(), Shape::Sphere(1000.0))?;
		let zone = Zone::create(&zone_spatial, Transform::identity(), &zone_field)?;
		Ok(ZoneMovement {
			velocity: Vec3::ZERO,
			zone,
			_zone_field: zone_field,
			zone_spatial,
			captured: HashMap::new(),
			entered: HashMap::new(),
		})
	}
}
