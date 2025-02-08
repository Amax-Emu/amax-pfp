use blur_plugins_core::{BlurAPI, BlurPlugin};
use simplelog::*;
use std::{ffi::c_void, sync::LazyLock};

use windows::{core::PCSTR, Win32::System::LibraryLoader::GetModuleHandleA};

mod d3d9_utils;
mod gamer_picture_manager;
mod img_preprocess;

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
			cfg,
			TerminalMode::Mixed,
			ColorChoice::Auto,
		),
		WriteLogger::new(LevelFilter::Trace, Config::default(), log_file),
	])
	.unwrap();
	log_panics::init();
}

#[allow(dead_code)]
struct CoolBlurPlugin {
	api: &'static mut dyn BlurAPI,
	ptr_base: *mut c_void,
}

impl CoolBlurPlugin {
	fn new(api: &'static mut dyn BlurAPI) -> Self {
		let ptr_base = Self::get_exe_base_ptr();
		unsafe {
			let pp = ptr_base as usize; // very ugly
			assert!(pp == 0x00400000);
			gamer_picture_manager::create_get_primary_profile_picture_hook();

			std::thread::spawn(move || {
				gamer_picture_manager::remote_pfp_updater(pp as _);
			});
		};

		Self { api, ptr_base }
	}


	/// Just for util
	pub fn get_exe_base_ptr() -> *mut c_void {
		static ONCE: LazyLock<usize> = LazyLock::new(|| {
			let ptr_base: *mut c_void = unsafe { GetModuleHandleA(PCSTR::null()) }.unwrap().0 as _;
			ptr_base as usize
		});
		let p: usize = *ONCE;
		assert!(p == 0x00400000);
		p as *mut c_void
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
	Box::new(CoolBlurPlugin::new(api))
}
