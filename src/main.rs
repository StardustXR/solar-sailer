mod input;
mod mode_button;
mod monado_movement;
mod reparentable_movement;
mod solar_sailer;

use glam::Vec3;
use input::Input;
use libmonado::Monado;
use monado_movement::MonadoMovement;
use reparentable_movement::ReparentMovement;
use solar_sailer::{Mode, SolarSailer};
use stardust_xr_fusion::{
	client::Client,
	core::schemas::zbus::{Connection, conn::Builder, fdo::ObjectManager},
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
	let conn = Builder::session()
		.unwrap()
		.serve_at("/", ObjectManager)
		.unwrap()
		.build()
		.await
		.unwrap();
	let object_registry = ObjectRegistry::new(&conn).await;
	let client_handle = client.handle();
	let async_loop = client.async_event_loop();
	let client = client_handle;

	// let mut button_hand = ModeButton::new(&client, ButtonLocation::Hand).await;
	// let mut button_controller = ModeButton::new(&client, ButtonLocation::Controller).await;

	let input = Input::new_pen(&client, conn.clone()).await.unwrap();

	let mut solar_sailer = SolarSailer::new(client.clone(), object_registry, input).await;

	let event_handle = async_loop.get_event_handle();
	loop {
		event_handle.wait().await;
		let Some(event) = client.get_root().recv_root_event() else {
			continue;
		};
		match event {
			RootEvent::Ping { response } => response.send_ok(()),
			RootEvent::Frame { info } => {
				solar_sailer.handle_input();
				let switch_mode = solar_sailer.should_switch_mode();
				// if switch_mode {
				// 	solar_sailer.mode = match solar_sailer.mode {
				// 		Mode::Disabled => Mode::MonadoOffset,
				// 		Mode::MonadoOffset => Mode::Zone,
				// 		Mode::Zone => Mode::Disabled,
				// 	};
				// }
				if switch_mode {
					solar_sailer.switch_mode(match solar_sailer.current_mode() {
						Mode::Reparent => Mode::MonadoOffset,
						Mode::MonadoOffset => Mode::Reparent,
						Mode::Disabled => Mode::MonadoOffset,
					});
				}

				solar_sailer.update_signifiers();
				solar_sailer.update_velocity(info.delta).await;
				solar_sailer.apply_offset(info.delta).await;
			}
			RootEvent::SaveState { response: _ } => {}
		}
	}
}
