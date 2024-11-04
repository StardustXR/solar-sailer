mod solar_sailer;

use std::path::Path;

use libmonado::Monado;
use manifest_dir_macros::directory_relative_path;
use solar_sailer::SolarSailer;
use stardust_xr_fusion::{
	client::Client,
	root::{RootAspect, RootEvent},
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
	color_eyre::install().unwrap();
	let mut client = Client::connect().await.unwrap();
	client
		.setup_resources(&[Path::new(directory_relative_path!("res"))])
		.unwrap();

	let monado = Monado::auto_connect().expect("Couldn't connect to monado :(");

	let mut solar_sailer = SolarSailer::new(monado, client.handle(), 0.005).unwrap();
	client
		.sync_event_loop(|client, _| {
			while let Some(event) = client.get_root().recv_root_event() {
				match event {
					RootEvent::Frame { info } => {
						solar_sailer.frame(info);
					}
					RootEvent::SaveState { response } => {
						response.send(solar_sailer.save_state());
					}
				}
			}
		})
		.await
		.unwrap();
}
