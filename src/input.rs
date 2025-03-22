use std::{f32::consts::FRAC_PI_2, sync::Arc};

use glam::{vec3, Mat4, Vec3};
use stardust_xr_fusion::{
	drawable::{Line, Lines, LinesAspect as _},
	fields::{CylinderShape, Field, Shape},
	input::{InputData, InputDataType, InputHandler},
	node::NodeResult,
	objects::hmd,
	spatial::Transform,
	values::color::rgba_linear,
	ClientHandle,
};
use stardust_xr_molecules::{
	input_action::{InputQueue, InputQueueable as _, SingleAction},
	lines::{circle, LineExt as _},
};

use crate::solar_sailer::Mode;

pub struct PenInput {}
pub enum Input {
	Grab(GrabInput),
	Pen(PenInput),
}
pub struct GrabInput {
	pub move_action: SingleAction,
	pub _field: Field,
	pub queue: InputQueue,
	pub prev_position: Option<Vec3>,
	pub signifiers: Lines,
}

impl Input {
	pub async fn new_grab(client: &Arc<ClientHandle>) -> NodeResult<Self> {
		let field = Field::create(
			&hmd(client).await.unwrap(),
			Transform::identity(),
			Shape::Cylinder(CylinderShape {
				length: 0.0,
				radius: 0.0,
			}),
		)
		.unwrap();
		let queue = InputHandler::create(&field, Transform::identity(), &field)?.queue()?;
		Ok(Input::Grab(GrabInput {
			signifiers: Lines::create(queue.handler(), Transform::identity(), &[]).unwrap(),
			move_action: SingleAction::default(),
			_field: field,
			queue,
			prev_position: None,
		}))
	}
}
impl Input {
	pub fn handle_input(&mut self) {
		match self {
			Input::Grab(grab_input) => grab_input.handle_input(),
			Input::Pen(pen_input) => todo!(),
		}
	}
	pub fn waft(&mut self, delta_secs: f32) -> Vec3 {
		match self {
			Input::Grab(grab_input) => grab_input.waft(delta_secs),
			Input::Pen(pen_input) => todo!(),
		}
	}
	pub fn update_signifiers(&self, mode: Mode) {
		match self {
			Input::Grab(grab_input) => grab_input.update_signifiers(mode),
			Input::Pen(pen_input) => todo!(),
		}
	}
}
impl GrabInput {
	pub fn handle_input(&mut self) {
		self.queue.handle_events();
		self.move_action.update(
			true,
			&self.queue,
			|data| !matches!(&data.input, InputDataType::Pointer(_)),
			|data| {
				println!("test");
				data.datamap.with_data(|d| match &data.input {
					InputDataType::Hand(_) => d.idx("grab_strength").as_f32() > 0.9,
					_ => d.idx("grab").as_f32() > 0.9,
				})
			},
		);
	}
	pub fn waft(&mut self, delta_secs: f32) -> Vec3 {
		println!("waft");
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
	pub fn update_signifiers(&self, mode: Mode) {
		if matches!(mode, Mode::Disabled) {
			self.signifiers.set_lines(&[]).unwrap();
			return;
		}
		let mut signifier_lines = self
			.move_action
			.hovering()
			.current()
			.iter()
			.map(|input| self.generate_signifier(input, false, mode))
			.collect::<Vec<_>>();
		signifier_lines.extend(
			self.move_action
				.actor()
				.map(|input| self.generate_signifier(input, true, mode)),
		);
		println!("lines: {}", signifier_lines.len());
		self.signifiers.set_lines(&signifier_lines).unwrap();
	}
	fn generate_signifier(&self, input: &InputData, grabbing: bool, mode: Mode) -> Line {
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
			line.color(rgba_linear!(0.0, 0.549, 1.0, 1.0))
		} else if matches!(mode, Mode::MonadoOffset) {
			line.color(rgba_linear!(1.0, 1.0, 0.0, 1.0))
		} else {
			line
		}
	}
}
