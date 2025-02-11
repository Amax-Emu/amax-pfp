use blur_plugins_core::{BlurAPI, BlurPlugin};
use log::LevelFilter;
use simplelog::{
	ColorChoice, CombinedLogger, ConfigBuilder, TermLogger, TerminalMode, WriteLogger,
};
use std::{ffi::c_void, sync::LazyLock};

use windows::{core::PCSTR, Win32::System::LibraryLoader::GetModuleHandleA};

mod d3d9_utils;
mod gamer_picture_manager;
mod img_preprocess;
mod ll_crimes;

#[allow(dead_code)]
pub struct CoolBlurPlugin {}

static mut G_API: Option<&dyn BlurAPI> = None;

impl CoolBlurPlugin {
	fn new(_api: &dyn BlurAPI) -> Self {
		let ptr_base = Self::get_exe_base_ptr();
		gamer_picture_manager::install_hook_request_remote_picture(ptr_base); // does this do anything?
		gamer_picture_manager::install_hook_get_primary_profile_picture_v2(ptr_base);
		std::thread::spawn(ll_crimes::run);
		Self {}
	}

	/// Just for util
	pub fn get_exe_base_ptr() -> *mut c_void {
		//NOTE: using LazyLock for this is kinda stupid
		// I just have a weird personal vendetta against GetModuleHandleA(..)
		static ONCE: LazyLock<usize> = LazyLock::new(|| {
			let ptr_base: *mut c_void = unsafe { GetModuleHandleA(PCSTR::null()) }.unwrap().0 as _;
			assert!(ptr_base as usize == 0x00400000);
			ptr_base as usize
		});
		let p: usize = *ONCE;
		p as *mut c_void
	}

	pub fn get_api() -> &'static dyn BlurAPI {
		unsafe { G_API.unwrap() }
	}
}

impl BlurPlugin for CoolBlurPlugin {
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
	Box::new(CoolBlurPlugin::new(api))
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
	log_panics::Config::new().install_panic_hook();
}
