use std::{collections::HashMap, sync::Arc};

use glam::{Affine3A, Vec3};
use stardust_xr_fusion::{
	ClientHandle,
	node::NodeResult,
	objects::{ObjectInfo, interfaces::ReparentableProxy, object_registry::ObjectRegistry},
	query::{ObjectQuery, QueryEvent},
	spatial::{Spatial, SpatialAspect, SpatialRef, SpatialRefAspect, Transform},
};
use stardust_xr_molecules::dbus::AbortOnDrop;
use tracing::error;

use crate::solar_sailer::mat_from_transform;

pub struct ReparentMovement {
	pub velocity: Vec3,
	spatial: Spatial,
	spatial_id: u64,
	reparenting: Option<AbortOnDrop>,
	obj_reg: Arc<ObjectRegistry>,
}

impl ReparentMovement {
	pub async fn apply_offset(&mut self, delta_secs: f32, velocity_ref: &SpatialRef) {
		if self.velocity.length_squared() < 0.0005 {
			return;
		}
		match (self.velocity == Vec3::ZERO, self.reparenting.is_some()) {
			(true, true) => {
				self.reparenting.take();
			}
			(false, false) => {
				self.reparenting = Some(AbortOnDrop(
					tokio::spawn(Self::reparent_task(self.spatial_id, self.obj_reg.clone()))
						.abort_handle(),
				));
			}
			_ => {}
		}
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

	pub async fn new(client: &Arc<ClientHandle>, obj_reg: Arc<ObjectRegistry>) -> NodeResult<Self> {
		let spatial = Spatial::create(client.get_root(), Transform::identity(), false)?;
		let spatial_id = spatial.export_spatial().await?;
		Ok(ReparentMovement {
			velocity: Vec3::ZERO,
			spatial,
			spatial_id,
			obj_reg,
			reparenting: None,
		})
	}

	async fn reparent_task(spatial_id: u64, obj_reg: Arc<ObjectRegistry>) {
		let mut reparented = ReparentedSpatials::default();
		let mut query = ObjectQuery::<ReparentableProxy, ()>::new(obj_reg, ());
		while let Some(e) = query.recv_event().await {
			match e {
				QueryEvent::NewMatch(object_info, proxy) => {
					if proxy.parent(spatial_id).await.is_ok() {
						reparented.0.insert(object_info, proxy);
					}
				}
				QueryEvent::MatchLost(object_info) => {
					reparented.0.remove(&object_info);
				}
				_ => {}
			}
		}
	}
}

#[derive(Default)]
struct ReparentedSpatials<'a>(HashMap<ObjectInfo, ReparentableProxy<'a>>);
impl Drop for ReparentedSpatials<'_> {
	fn drop(&mut self) {
		tokio::runtime::Handle::current().block_on(async {
			for (_, proxy) in self.0.drain() {
				_ = proxy.unparent().await;
			}
		});
	}
}
