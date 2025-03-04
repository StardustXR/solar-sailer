use std::sync::Arc;

use glam::Vec3;
use stardust_xr_fusion::{
	drawable::Line,
	fields::{Field, Shape},
	input::{InputData, InputDataType, InputHandler},
	node::NodeResult,
	objects::hmd,
	spatial::Transform,
	ClientHandle,
};
use stardust_xr_molecules::input_action::{InputQueue, InputQueueable as _, SingleAction};

pub struct Input {
	pub move_action: SingleAction,
	pub _field: Field,
	pub queue: InputQueue,
	pub prev_position: Option<Vec3>,
}

impl Input {
	pub async fn new(client: &Arc<ClientHandle>) -> NodeResult<Self> {
		let field = Field::create(
			&hmd(client).await.unwrap(),
			Transform::identity(),
			Shape::Sphere(0.0),
		)?;
		let queue = InputHandler::create(&field, Transform::identity(), &field)?.queue()?;
		Ok(Input {
			move_action: SingleAction::default(),
			_field: field,
			queue,
			prev_position: None,
		})
	}
	pub fn handle_input(&mut self) {
		self.queue.handle_events();
		self.move_action.update(
			true,
			&self.queue,
			|data| !matches!(&data.input, InputDataType::Pointer(_)),
			|data| {
				data.datamap.with_data(|d| match &data.input {
					InputDataType::Hand(_) => d.idx("grab_strength").as_f32() > 0.9,
					_ => d.idx("grab").as_f32() > 0.9,
				})
			},
		);
	}
	pub fn waft(&mut self, delta_secs: f32) -> Vec3 {
		let position = self.move_action.actor().map(|p| match &p.input {
			InputDataType::Hand(h) => h.palm.position.into(),
			InputDataType::Tip(t) => t.origin.into(),
			_ => unreachable!(),
		});

		if let Some(prev_position) = self.prev_position {
			if let Some(position) = position {
				let offset: Vec3 = position - prev_position;
				let offset_magnify = (offset.length() * delta_secs).powf(0.9);
				return offset.normalize_or_zero() * offset_magnify;
			}
		}

		self.prev_position = position;
		Vec3::ZERO
	}
	pub fn update_signifiers(&self, mut gen: impl FnMut(&InputData, bool) -> Line) -> Vec<Line> {
		let mut signifier_lines = self
			.move_action
			.hovering()
			.current()
			.iter()
			.map(|input| gen(input, false))
			.collect::<Vec<_>>();
		signifier_lines.extend(self.move_action.actor().map(|input| gen(input, true)));
		signifier_lines
	}
}
