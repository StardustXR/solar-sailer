mod input;
mod mode_button;
mod monado_movement;
mod reparentable_movement;
mod solar_sailer;

use input::Input;
use libmonado::Monado;
use monado_movement::MonadoMovement;
use reparentable_movement::ReparentMovement;
use solar_sailer::{Mode, SolarSailer};
use stardust_xr_fusion::{
	client::Client,
	core::schemas::zbus::Connection,
	objects::object_registry::ObjectRegistry,
	project_local_resources,
	root::{RootAspect, RootEvent},
};
use tracing::error;

#[tokio::main(flavor = "current_thread")]
async fn main() {
	tracing_subscriber::fmt().pretty().with_file(false).init();
	let client = Client::connect().await.unwrap();
	client
		.setup_resources(&[&project_local_resources!("res")])
		.unwrap();
	let conn = Connection::session().await.unwrap();
	let object_registry = ObjectRegistry::new(&conn).await;
	let client_handle = client.handle();
	let async_loop = client.async_event_loop();
	let client = client_handle;

	let monado = match Monado::auto_connect() {
		Ok(v) => Some(v),
		Err(err) => {
			error!("Couldn't connect to monado :( {err}");
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
		reparent_movement: ReparentMovement::new(&client, object_registry).unwrap(),
	};

	let event_handle = async_loop.get_event_handle();
	loop {
		event_handle.wait().await;
		let Some(event) = client.get_root().recv_root_event() else {
			continue;
		};
		match event {
			RootEvent::Ping { response } => response.send_ok(()),
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
						Mode::Reparent => Mode::MonadoOffset,
						Mode::MonadoOffset => Mode::Reparent,
						Mode::Disabled => Mode::MonadoOffset,
					};
				}

				solar_sailer.handle_input();
				solar_sailer.update_signifiers();
				solar_sailer.update_velocity(info.delta).await;
				solar_sailer.apply_offset(info.delta).await;
			}
			RootEvent::SaveState { response: _ } => {}
		}
	}
}
