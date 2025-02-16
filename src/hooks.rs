use crate::downloader::get_primary_profile_img_data;
use std::ffi::c_void;

use retour::static_detour;

use crate::{data::gamer_picture_manager::GamerPictureManager, MyPlugin};

static_detour! {
	static GetPrimaryProfilePictureHook: unsafe extern "system" fn() -> bool;
}

// static_detour! { static GamePictureManager_CreateHook: unsafe extern "system" fn(i32,i32,*const [u8;32],bool) -> bool; }

static_detour! {
	static GamePictureManager_RequestRemotePicture: unsafe extern "system" fn(i32) -> bool;
}

//0079da10
// little pesky function messing up things
// static_detour! { static GamePictureManager_WipeRemotePictures: unsafe extern "fastcall" fn(*mut GamerPictureManager); }

pub fn install_hook_get_primary_profile_picture_v2(ptr_base: *mut c_void) {
	type FnGetPrimaryProfilePicture = unsafe extern "system" fn() -> bool;
	const ORG_FN_ADDRESS_OFFSET_GET_PRIMARY_PROFILE_PICTURE: isize = 0x0095E170;
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET_GET_PRIMARY_PROFILE_PICTURE);
	unsafe {
		let ptr = std::mem::transmute::<*mut c_void, FnGetPrimaryProfilePicture>(ptr);
		GetPrimaryProfilePictureHook
			.initialize(ptr, get_primary_profile_picture_hook)
			.unwrap()
			.enable()
			.unwrap();
	}
}

pub fn install_hook_request_remote_picture(ptr_base: *mut c_void) {
	type FnRequestRemotePicture = unsafe extern "system" fn(i32) -> bool;
	const ORG_FN_ADDRESS_OFFSET_REQUEST_REMOTE_PICTURE: isize = 0x786D20;
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET_REQUEST_REMOTE_PICTURE);
	unsafe {
		let ptr = std::mem::transmute::<*mut c_void, FnRequestRemotePicture>(ptr);
		GamePictureManager_RequestRemotePicture
			.initialize(ptr, request_remote_picture_hook)
			.unwrap()
			.enable()
			.unwrap();
	}
}

fn get_primary_profile_picture_hook() -> bool {
	log::trace!("GetPrimaryProfilePictureHook!");
	// This hook gets called from the main thread
	// Getting the img_data in the main thread would freeze game until response or timeout.
	// We spawn a thread to do all of that in the background.
	std::thread::Builder::new()
		.name("AMAX_PFP_Primary_profile_fetcher".to_string())
		.spawn(move || {
			let ptr_base = MyPlugin::get_exe_base_ptr();
			let local_picures = unsafe {
				*(GamerPictureManager::summon(ptr_base)
					.unwrap()
					.local_pictures_ptr)
			};
			for local_gamer_pic in local_picures {
				let local_gamer_pic = unsafe { &mut *local_gamer_pic };
				if local_gamer_pic.get_name() == "GAMERPIC_0" {
					let username = MyPlugin::get_api().get_saved_profile_username();
					//let img_data = get_image_from_url("https://cdn.discordapp.com/avatars/925665499692544040/483eb1b92db6a449a0e2bed9a8b48bb3.png");
					log::info!("Loading primary profile picture for \"{username}\"...");
					let mut img_data = match get_primary_profile_img_data(&username) {
						Ok(img_data) => img_data,
						_ => return,
					};
					local_gamer_pic.texture_ptr = crate::tex::create_64x64_d3d9tex(&mut img_data); // YAY?
					local_gamer_pic.active = true;
					local_gamer_pic.free = false;
					log::info!("We set the primary profile pic!!!! (?)");
				}
			}
		})
		.expect("Not able to create thread?");

	false
}

/// When does this thing even run?
fn request_remote_picture_hook(arg: i32) -> bool {
	type FnRequestRemotePicture = unsafe extern "system" fn(i32) -> bool;
	const ORG_FN_ADDRESS_OFFSET_REQUEST_REMOTE_PICTURE: isize = 0x786D20; // Yea Yea Yea we should save the original somewhere.
	let ptr_base = MyPlugin::get_exe_base_ptr();
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET_REQUEST_REMOTE_PICTURE);
	unsafe {
		let fn_org = std::mem::transmute::<*mut c_void, FnRequestRemotePicture>(ptr);
		let org_result = fn_org(arg);
		log::trace!("GamePictureManager_RequestRemotePicture({arg}) -> {org_result}");
		org_result
	}
}

pub fn trigger_lobby_update_v2(ptr_base: *mut c_void) {
	/// @Aibot: How did you find these?
	const OFFSET_PTR_LOBBY_START: isize = 0x00E42FF8;
	/// I want to document <what> they actually are, and give them better names
	const OFFSET_TRIGGER_UPDATE_BOOL: isize = 0x181;
	let p: *mut *mut bool = ptr_base.wrapping_byte_offset(OFFSET_PTR_LOBBY_START) as _;
	unsafe {
		let p: *mut bool = p.read();
		if p.is_null() {
			log::trace!("trigger_lobby_update_v2() failed (start pointer is null).");
			return;
		}
		log::trace!("Triggering lobby update!");
		p.wrapping_byte_offset(OFFSET_TRIGGER_UPDATE_BOOL)
			.write(true);
	}
}
