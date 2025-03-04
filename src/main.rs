mod input;
mod monado_movement;
// mod shared;
mod mode_button;
mod solar_sailer;
mod zone_movement;

use input::Input;
use libmonado::Monado;
use manifest_dir_macros::directory_relative_path;
use mode_button::{ButtonLocation, ModeButton};
use monado_movement::MonadoMovement;
use solar_sailer::{Mode, SolarSailer};
use stardust_xr_fusion::{
	client::Client,
	drawable::Lines,
	objects::hmd,
	root::{RootAspect, RootEvent},
	spatial::Transform,
	values::color::rgba_linear,
};
use zone_movement::ZoneMovement;

#[tokio::main(flavor = "current_thread")]
async fn main() {
	color_eyre::install().unwrap();
	let client = Client::connect().await.unwrap();
	let client_handle = client.handle();
	let async_loop = client.async_event_loop();
	client_handle
		.get_root()
		.set_base_prefixes(&[directory_relative_path!("res").to_string()])
		.unwrap();
	let client = client_handle;

	let monado = match Monado::auto_connect() {
		Ok(v) => Some(v),
		Err(err) => {
			println!("Couldn't connect to monado :( {err}");
			None
		}
	};

	// let mut button_hand = ModeButton::new(&client, ButtonLocation::Hand).await;
	// let mut button_controller = ModeButton::new(&client, ButtonLocation::Controller).await;
	let mut button_hand: Option<ModeButton> = None;
	let mut button_controller: Option<ModeButton> = None;

	let input = Input::new(&client).await.unwrap();

	let hmd = hmd(&client).await.unwrap();

	let mut solar_sailer = SolarSailer {
		monado_movement: MonadoMovement::from_monado(&client, monado).await,
		signifiers: Lines::create(input.queue.handler(), Transform::identity(), &[]).unwrap(),
		mode: Mode::MonadoOffset,
		grab_color: rgba_linear!(0.0, 0.549, 1.0, 1.0),
		input,
		zone_movement: ZoneMovement::new(&client).unwrap(),
		hmd,
	};

	let event_handle = async_loop.get_event_handle();
	loop {
		event_handle.wait().await;
		let Some(event) = client.get_root().recv_root_event() else {
			continue;
		};
		if let RootEvent::Frame { info } = event {
			let mut switch_mode = false;
			if let Some(button) = button_hand.as_mut() {
				switch_mode |= button.update();
			}
			if let Some(button) = button_controller.as_mut() {
				switch_mode |= button.update();
			}

			if switch_mode {
				solar_sailer.mode = match solar_sailer.mode {
					Mode::Disabled => Mode::MonadoOffset,
					Mode::MonadoOffset => Mode::Zone,
					Mode::Zone => Mode::Disabled,
				};
			}

			solar_sailer.handle_events();
			solar_sailer.handle_input();
			solar_sailer.update_signifiers();
			solar_sailer.update_velocity(info.delta);
			solar_sailer.apply_offset(info.delta).await;
		}
	}
}
