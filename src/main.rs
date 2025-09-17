mod input;
mod mode_button;
mod monado_movement;
mod solar_sailer;
mod zone_movement;

use input::Input;
use libmonado::Monado;
use manifest_dir_macros::directory_relative_path;
use monado_movement::MonadoMovement;
use solar_sailer::{Mode, SolarSailer};
use stardust_xr_fusion::{
	client::Client,
	root::{RootAspect, RootEvent},
};
use zone_movement::ZoneMovement;

#[tokio::main(flavor = "current_thread")]
async fn main() {
	tracing_subscriber::fmt().pretty().finish();
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

	let input = Input::new_pen(&client).await.unwrap();

	let mut solar_sailer = SolarSailer {
		monado_movement: MonadoMovement::from_monado(&client, monado).await,
		mode: Mode::MonadoOffset,
		input,
		// zone_movement: ZoneMovement::new(&client).unwrap(),
	};

	let event_handle = async_loop.get_event_handle();
	loop {
		event_handle.wait().await;
		let Some(event) = client.get_root().recv_root_event() else {
			continue;
		};
		match event {
			RootEvent::Ping { response } => response.send(Ok(())),
			RootEvent::Frame { info } => {
				let switch_mode = solar_sailer.input.update_mode();
				// if switch_mode {
				// 	solar_sailer.mode = match solar_sailer.mode {
				// 		Mode::Disabled => Mode::MonadoOffset,
				// 		Mode::MonadoOffset => Mode::Zone,
				// 		Mode::Zone => Mode::Disabled,
				// 	};
				// }
				if switch_mode {
					solar_sailer.mode = match solar_sailer.mode {
						Mode::Zone => Mode::MonadoOffset,
						Mode::MonadoOffset => Mode::Zone,
						Mode::Disabled => Mode::MonadoOffset,
					};
				}

				solar_sailer.handle_events();
				solar_sailer.handle_input();
				solar_sailer.update_signifiers();
				solar_sailer.update_velocity(info.delta).await;
				solar_sailer.apply_offset(info.delta).await;
			}
			RootEvent::SaveState { response: _ } => {}
		}
	}
}
