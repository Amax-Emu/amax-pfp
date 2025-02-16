use blur_plugins_core::{BlurAPI, BlurPlugin};
use log::LevelFilter;
use simplelog::{
	ColorChoice, CombinedLogger, ConfigBuilder, TermLogger, TerminalMode, WriteLogger,
};
use std::ffi::c_void;

mod data;
mod downloader;
mod hooks;
mod tex;
mod updater;

pub struct MyPlugin {}

static mut G_API: Option<&dyn BlurAPI> = None;

impl MyPlugin {
	fn new(api: &dyn BlurAPI) -> Self {
		let ptr_base = api.get_exe_base_ptr();
		hooks::install_hook_request_remote_picture(ptr_base); // does this do anything?
		hooks::install_hook_get_primary_profile_picture_v2(ptr_base);
		std::thread::spawn(updater::Updater::run);
		Self {}
	}

	pub fn get_api() -> &'static dyn BlurAPI {
		unsafe { G_API.unwrap() }
	}

	pub fn get_exe_base_ptr() -> *mut c_void {
		Self::get_api().get_exe_base_ptr()
	}
}

impl BlurPlugin for MyPlugin {
	fn name(&self) -> &'static str {
		"AMAX_PFP"
	}

	fn on_event(&self, event: &blur_plugins_core::BlurEvent) {
		log::trace!("AMAX_PFP: on_event({event:?})");
	}

	fn free(&self) {
		log::trace!("AMAX_PFP: Unloading!");
	}
}

#[no_mangle]
fn plugin_init(api: &'static mut dyn BlurAPI) -> Box<dyn BlurPlugin> {
	init_logs();
	unsafe {
		G_API = Some(api);
	}
	Box::new(MyPlugin::new(api))
}

fn init_logs() {
	let cfg = ConfigBuilder::new()
		.set_time_offset_to_local()
		.unwrap()
		.add_filter_allow_str("amax_pfp")
		.build();

	let log_file = blur_plugins_core::create_log_file("amax_pfp.log").unwrap();

	CombinedLogger::init(vec![
		TermLogger::new(
			LevelFilter::Trace,
			cfg.clone(),
			TerminalMode::Mixed,
			ColorChoice::Auto,
		),
		WriteLogger::new(LevelFilter::Trace, cfg, log_file),
	])
	.unwrap();
	log_panics::init();
}
