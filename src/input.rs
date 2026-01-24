use std::{f32::consts::FRAC_PI_2, sync::Arc};

use glam::{Mat4, Quat, Vec3, vec3};
use stardust_xr_fusion::{
	ClientHandle,
	drawable::{Line, LinePoint, Lines, LinesAspect as _, Model},
	fields::{CylinderShape, Field, Shape},
	input::{InputData, InputDataType, InputHandler},
	node::NodeResult,
	objects::hmd,
	spatial::{Spatial, SpatialAspect as _, SpatialRef, SpatialRefAspect, Transform},
	values::{ResourceID, color::rgba_linear},
	zbus::Connection,
};
use stardust_xr_molecules::{
	UIElement,
	button::{Button, ButtonSettings},
	input_action::{InputQueue, InputQueueable as _, SimpleAction, SingleAction},
	lines::{LineExt as _, circle},
	reparentable::Reparentable,
};
use tracing::error;

use crate::{
	mode_button::ModeButton,
	solar_sailer::{Mode, mat_from_transform},
};

pub struct PenInput {
	move_action: SimpleAction,
	grab_action: SingleAction,
	field: Field,
	pen_root: Spatial,
	queue: InputQueue,
	prev_position: Option<Vec3>,
	signifiers: Lines,
	client: Arc<ClientHandle>,
	button: Button,
	reparentable: Option<Reparentable>,
	connection: Connection,
	_button_model: Model,
}
#[allow(dead_code, clippy::large_enum_variant)]
pub enum Input {
	Grab(GrabInput),
	Pen(PenInput),
}
pub struct GrabInput {
	move_action: SingleAction,
	_field: Field,
	queue: InputQueue,
	prev_position: Option<Vec3>,
	signifiers: Lines,
	client: Arc<ClientHandle>,
	button_hand: Option<ModeButton>,
	button_controller: Option<ModeButton>,
}

impl Input {
	pub async fn new_pen(client: &Arc<ClientHandle>, connection: Connection) -> NodeResult<Self> {
		PenInput::new(client, connection).await.map(Input::Pen)
	}
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
			client: client.clone(),
			button_hand: None,
			button_controller: None,
		}))
	}
}
impl Input {
	pub fn update_mode(&mut self) -> bool {
		match self {
			Input::Grab(grab_input) => grab_input.update_mode(),
			Input::Pen(pen_input) => pen_input.update_mode(),
		}
	}
	pub fn handle_input(&mut self) {
		match self {
			Input::Grab(grab_input) => grab_input.handle_input(),
			Input::Pen(pen_input) => pen_input.handle_input(),
		}
	}
	pub async fn waft(&mut self, delta_secs: f32) -> Vec3 {
		match self {
			Input::Grab(grab_input) => grab_input.waft(delta_secs).await,
			Input::Pen(pen_input) => pen_input.waft(delta_secs).await,
		}
	}
	pub fn update_signifiers(&self, mode: Mode) {
		match self {
			Input::Grab(grab_input) => grab_input.update_signifiers(mode),
			Input::Pen(pen_input) => pen_input.update_signifiers(mode),
		}
	}
	pub fn get_velocity_space(&self) -> SpatialRef {
		match self {
			Input::Grab(grab_input) => grab_input.client.get_root().clone().as_spatial_ref(),
			Input::Pen(pen_input) => pen_input.client.get_root().clone().as_spatial_ref(),
		}
	}
}
impl PenInput {
	const LENGTH: f32 = 0.075;
	const THICKNESS: f32 = 0.005;
	fn update_mode(&mut self) -> bool {
		if !self.button.handle_events() {
			return false;
		}
		self.button.released()
	}
	async fn new(client: &Arc<ClientHandle>, connection: Connection) -> NodeResult<Self> {
		let pen_root = Spatial::create(client.get_root(), Transform::none())?;
		let signifiers = Lines::create(&pen_root, Transform::none(), &[])?;
		let field = Field::create(
			&pen_root,
			Transform::from_translation([0.0, Self::LENGTH * 0.5, 0.0]),
			Shape::Cylinder(CylinderShape {
				length: Self::LENGTH,
				radius: Self::THICKNESS * 0.5,
			}),
		)?;
		let queue = InputHandler::create(client.get_root(), Transform::none(), &field)?.queue()?;

		let button = Button::create(
			&pen_root,
			Transform::from_translation_rotation(
				[0.0, Self::LENGTH * 1.1, 0.0],
				Quat::from_rotation_x(-FRAC_PI_2),
			),
			[0.02; 2],
			ButtonSettings::default(),
		)?;
		let button_model = Model::create(
			button.touch_plane().root(),
			Transform::identity(),
			&ResourceID::new_namespaced("solar_sailer", "move_icon"),
		)?;

		let mut pen = Self {
			move_action: Default::default(),
			grab_action: Default::default(),
			field,
			pen_root,
			queue,
			prev_position: None,
			signifiers,
			client: client.clone(),
			button,
			reparentable: None,
			connection,

			_button_model: button_model,
		};
		pen.make_reparentable();
		Ok(pen)
	}
	fn make_reparentable(&mut self) {
		if self.reparentable.is_some() {
			return;
		}
		self.reparentable = Reparentable::create(
			self.connection.clone(),
			"/Pen",
			self.queue.handler().clone().as_spatial_ref(),
			self.pen_root.clone(),
			Some(self.field.clone()),
		)
		.inspect_err(|err| error!("unable to make reparentable: {err}"))
		.ok();
	}
	fn handle_input(&mut self) {
		if !self.queue.handle_events() {
			return;
		}
		self.grab_action.update(
			false,
			&self.queue,
			|data| data.distance < 0.05,
			|data| {
				data.datamap.with_data(|datamap| match &data.input {
					InputDataType::Hand(_) => datamap.idx("grab_strength").as_f32() > 0.80,
					InputDataType::Tip(_) => datamap.idx("grab").as_f32() > 0.90,
					_ => false,
				})
			},
		);
		self.move_action.update(&self.queue, &|data| {
			data.datamap.with_data(|datamap| match &data.input {
				InputDataType::Hand(h) => {
					Vec3::from(h.thumb.tip.position).distance(h.index.tip.position.into()) < 0.03
				}
				InputDataType::Tip(_) => datamap.idx("select").as_f32() > 0.01,
				_ => false,
			})
		});

		if self.grab_action.actor_started() {
			self.reparentable.take();
		}
		if self.grab_action.actor_stopped() {
			self.make_reparentable();
		}
		let Some(grab_actor) = self.grab_action.actor() else {
			return;
		};
		let transform = match &grab_actor.input {
			InputDataType::Hand(h) => Transform::from_translation_rotation(
				(Vec3::from(h.thumb.tip.position) + Vec3::from(h.index.tip.position)) * 0.5,
				Quat::from(h.palm.rotation),
			),
			InputDataType::Tip(t) => Transform::from_translation_rotation(
				t.origin,
				Quat::from(t.orientation) * Quat::from_rotation_x(FRAC_PI_2),
			),
			_ => Transform::none(),
		};
		let _ = self
			.pen_root
			.set_relative_transform(self.queue.handler(), transform);
	}
	pub async fn waft(&mut self, _delta_secs: f32) -> Vec3 {
		let Some(grab_actor) = self.grab_action.actor() else {
			self.prev_position = None;
			return Vec3::ZERO;
		};
		let position = Vec3::from(match &grab_actor.input {
			InputDataType::Hand(h) => h.palm.position,
			InputDataType::Tip(t) => t.origin,
			_ => unreachable!(),
		});
		let handler_spatial = self.queue.handler().clone().as_spatial();

		let root_transform = handler_spatial
			.get_transform(self.client.get_root())
			.await
			.unwrap();
		let mat = mat_from_transform(&root_transform);
		let position = mat.transform_point3(position);
		if self.move_action.currently_acting().contains(grab_actor)
			&& let Some(prev_position) = self.prev_position
		{
			let offset: Vec3 = position - prev_position;
			let offset_magnify = (offset.length()/* * delta_secs */).powf(0.9);
			self.prev_position = Some(position);
			return offset.normalize_or_zero() * offset_magnify;
		}

		self.prev_position = Some(position);

		Vec3::ZERO
	}
	pub fn update_signifiers(&self, mode: Mode) {
		let thickness = Self::THICKNESS * 0.5;
		let visual_length = Self::LENGTH;
		let grabbing = self
			.grab_action
			.actor()
			.is_some_and(|actor| self.move_action.currently_acting().contains(actor));
		let color = match (mode, grabbing) {
			(Mode::Reparent, false) => rgba_linear!(1.0, 1.0, 1.0, 1.0),
			(Mode::MonadoOffset, false) => rgba_linear!(1.0, 1.0, 0.0, 1.0),
			(Mode::Disabled, _) => rgba_linear!(0.033104762, 0.033104762, 0.033104762, 1.),
			(_, true) => rgba_linear!(0., 0.26223028, 1., 1.),
		};
		let signifier_lines = [Line {
			points: vec![
				LinePoint {
					point: [0.0; 3].into(),
					thickness: 0.0,
					color,
				},
				LinePoint {
					point: [0.0, thickness, 0.0].into(),
					thickness,
					color,
				},
				LinePoint {
					point: [0.0, visual_length, 0.0].into(),
					thickness,
					color,
				},
			],
			cyclic: false,
		}];
		self.signifiers.set_lines(&signifier_lines).unwrap();
	}
}
impl GrabInput {
	fn update_mode(&mut self) -> bool {
		self.button_hand.as_mut().is_some_and(|b| b.update())
			|| self.button_controller.as_mut().is_some_and(|b| b.update())
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
	pub async fn waft(&mut self, _delta_secs: f32) -> Vec3 {
		let position = self.move_action.actor().map(|p| match &p.input {
			InputDataType::Hand(h) => h.palm.position.into(),
			InputDataType::Tip(t) => t.origin.into(),
			_ => unreachable!(),
		});

		if let Some(prev_position) = self.prev_position
			&& let Some(position) = position
		{
			let handler_spatial = self.queue.handler().clone().as_spatial();

			let root_transform = handler_spatial
				.get_transform(self.client.get_root())
				.await
				.unwrap();
			let mat = mat_from_transform(&root_transform);
			let position = mat.transform_point3(position);

			let offset: Vec3 = position - prev_position;
			let offset_magnify = (offset.length()/* * delta_secs */).powf(0.9);
			self.prev_position = Some(position);
			return offset.normalize_or_zero() * offset_magnify;
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
			line.color(rgba_linear!(0., 0.26223028, 1., 1.))
		} else if matches!(mode, Mode::MonadoOffset) {
			line.color(rgba_linear!(1.0, 1.0, 0.0, 1.0))
		} else {
			line
		}
	}
}
