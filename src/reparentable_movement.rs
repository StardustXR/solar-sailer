use std::{ops::Deref, sync::Arc};

use glam::{Affine3A, Vec3};
use stardust_xr_fusion::{
	ClientHandle,
	list_query::{ListEvent, ObjectListQuery},
	node::NodeResult,
	objects::{interfaces::ReparentableProxy, object_registry::ObjectRegistry},
	query::ObjectQuery,
	spatial::{Spatial, SpatialAspect, SpatialRef, SpatialRefAspect, Transform},
};
use tracing::{error, info};

use crate::solar_sailer::mat_from_transform;

pub struct ReparentMovement {
	pub velocity: Vec3,
	spatial: Spatial,
	moving: bool,
	query: ObjectListQuery<ReparentableProxy<'static>>,
}

impl ReparentMovement {
	pub async fn apply_offset(&mut self, delta_secs: f32, velocity_ref: &SpatialRef) {
		if self.velocity.length_squared() < 0.0005 {
			return;
		}
		match (self.velocity == Vec3::ZERO, self.moving) {
			(true, true) => {
				let map = self.query.iter().await;
				for reparentable in map.deref().values() {
					info!("unparenting reparentable");
					if let Err(err) = reparentable.unparent().await {
						error!("unable to unparent from reparentable: {err}");
					}
				}
			}
			(false, false) => {
				let map = self.query.iter().await;
				let Ok(id) = self
					.spatial
					.export_spatial()
					.await
					.inspect_err(|err| error!("unable to export Spatial: {err}"))
				else {
					return;
				};
				for reparentable in map.deref().values() {
					info!("reparenting reparentable");
					if let Err(err) = reparentable.parent(id).await {
						error!("unable to parent to reparentable: {err}");
					}
				}
			}
			_ => {}
		}
		self.moving = self.velocity != Vec3::ZERO;
		let mat = mat_from_transform(&velocity_ref.get_transform(&self.spatial).await.unwrap());
		let movement = mat.transform_vector3(self.velocity * delta_secs);
		let offset = Affine3A::from_translation(movement);
		if let Err(err) = self.spatial.set_relative_transform(
			velocity_ref,
			Transform::from_translation((offset * mat.inverse()).to_scale_rotation_translation().2),
		) {
			error!("unable to set transform: {err}");
		}
	}

	pub fn new(client: &Arc<ClientHandle>, obj_reg: Arc<ObjectRegistry>) -> NodeResult<Self> {
		let spatial = Spatial::create(client.get_root(), Transform::identity(), false)?;
		let (query, mapper) =
			ObjectQuery::<ReparentableProxy, ()>::new(obj_reg, ()).to_list_query();
		tokio::spawn(mapper.init(async |event| match event {
			ListEvent::NewMatch(v) => Some(v),
			ListEvent::Modified(v) => Some(v),
			_ => None,
		}));
		Ok(ReparentMovement {
			velocity: Vec3::ZERO,
			moving: false,
			spatial,
			query,
		})
	}
}
