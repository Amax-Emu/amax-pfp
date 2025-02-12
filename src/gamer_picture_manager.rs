use crate::img_preprocess::{get_amax_user_pfp_img_data, get_default_amax_pfp_img_data};
use crate::ll_crimes::get_gamer_picture_manager_v2;
use crate::CoolBlurPlugin;

use retour::static_detour;
use std::ffi::c_void;
use std::path::PathBuf;
use std::{io, str::Utf8Error};
use widestring::WideCString;
use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GamerPictureManager {
	thread: [u8; 20], // C8 AA EA 00 00 00 00 00 00 00 00 00 0D F0 AD BA 0D F0 AD BA || This is a thread pointer at PS3. I don't know what is it on PC.
	pub local_pictures_ptr: *const [*mut C_GamerPicture; 4],
	pub local_pictures_size: usize,
	pub local_pictures_len: usize, //this one is used in GetTotalPicturesFunctions
	pub remote_pictures_ptr: *const [*mut C_GamerPicture; 19], //hardcoding to 19
	pub remote_pictures_size: usize, // Has value of 26. Bug? Cut feature? We will never know.
	pub remote_pictures_len: usize, //this one is used in GetTotalPicturesFunctions
}

#[derive(Debug)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct C_GamerPicture {
	//total size on pc: 80
	unk_ptr0: u32, //0x4C 0xA8, 0xEA, 0x00,
	pub ref1: u16, // when this matches a NetRacer.mp_lobby_ref_id, the good things happen
	pub user_dw_id: u64,
	user_information: [u8; 8], // 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00
	pub active: bool,          // 0x00
	pub free: bool,            // 0x01
	pub gamer_pic_name: [u8; 30], //GAMERPIC_X or REMOTE_GAMERPIC_X
	size_as_big_end_temp: u32, // 0x00, 0x00, 0x00, 0x00
	unk_zeroes: u32,           // 0x00, 0x40 0x00, 0x00,
	unk_4_as_u16: u16,         //0x04, 0x00,
	pub texture_ptr: *mut IDirect3DTexture9, //0xE0, 0x71 0x90, 0x14
	pub default_texture_ptr: u32, //   0xB0, 0xCB 0x40, 0x0F
	unk4: u32,                 // 0x00, 0x00
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct NetPlayer {
	// this structure contain a lot of usefull data, but we're not interested
	unk0: [u8; 0x4],
	ptr_to_next: *mut NetPlayer, // 0x4
	unk1: [u8; 0x40],            // 0x4
	user_dw_id: u64,             //+0x48
	zeroes: [u8; 8],
	username_in_utf_16: [u16; 16], // +0x58
	unk2: [u8; 164],
	mp_lobby_ref_id: u8, //position of user in mplobby and the exact value remote_picture ref should be set to
	unk3: [u8; 107],
}

#[allow(unused)]
impl NetPlayer {
	pub fn get_next(&self) -> Option<&mut NetPlayer> {
		if self.ptr_to_next.is_null() {
			None
		} else {
			Some(unsafe { &mut *self.ptr_to_next })
		}
	}
	pub fn get_dw_id(&self) -> u64 {
		self.user_dw_id
	}
	pub fn get_username(&self) -> String {
		WideCString::from_vec_truncate(self.username_in_utf_16)
			.to_string()
			.unwrap()
	}

	pub fn set_dw_id(&mut self, x: u64) {
		self.user_dw_id = x;
	}

	pub fn get_lobby_ref(&self) -> u8 {
		self.mp_lobby_ref_id
	}
}

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
	const ORG_FN_ADDRESS_OFFSET: isize = 0x0095E170;
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET);
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

pub fn pretty_name(name_buf: &[u8]) -> String {
	let name = String::from_utf8(name_buf.to_vec()).unwrap();
	name.trim_matches(char::from(0)).to_string()
}

fn get_primary_profile_picture_hook() -> bool {
	log::trace!("GetPrimaryProfilePictureHook!");
	// NOTE: Getting the img_data in the main thread will freeze game until response or timeout.
	// So let us spawn a thread to do all of this in the background
	// This is not sound but I wanna try it
	std::thread::spawn(move || {
		let ptr_base = CoolBlurPlugin::get_exe_base_ptr();
		let local_picures = unsafe {
			*(get_gamer_picture_manager_v2(ptr_base)
				.unwrap()
				.local_pictures_ptr)
		};
		for local_gamer_pic in local_picures {
			let local_gamer_pic = unsafe { &mut *local_gamer_pic };
			let name = pretty_name(&local_gamer_pic.gamer_pic_name);
			if name == "GAMERPIC_0" {
				let username = match get_saved_profile_username_v2(ptr_base) {
					Ok(username) => username.to_string(),
					Err(e) => {
						log::error!(
								"Skipping primary profile picture setup because get_saved_profile_username() failed: {e}. "
							);
						continue;
					}
				};

				//let img_data = get_image_from_url("https://cdn.discordapp.com/avatars/925665499692544040/483eb1b92db6a449a0e2bed9a8b48bb3.png");
				log::info!("Loading primary profile picture for \"{username}\"...");
				let mut img_data = match get_primary_profile_img_data(&username) {
					Ok(img_data) => img_data,
					_ => return,
				};
				local_gamer_pic.texture_ptr =
					crate::d3d9_utils::create_64x64_d3d9tex(&mut img_data); // YAY?
				local_gamer_pic.active = true;
				local_gamer_pic.free = false;
				log::info!("We set the primary profile pic!!!! (?)");
			}
		}
	});

	false
}

fn request_remote_picture_hook(arg: i32) -> bool {
	type FnRequestRemotePicture = unsafe extern "system" fn(i32) -> bool;
	const ORG_FN_ADDRESS_OFFSET_REQUEST_REMOTE_PICTURE: isize = 0x786D20; // Yea Yea Yea we should save the original somewhere. Is this thing even called?
	let ptr_base = CoolBlurPlugin::get_exe_base_ptr();
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET_REQUEST_REMOTE_PICTURE);
	unsafe {
		let fn_org = std::mem::transmute::<*mut c_void, FnRequestRemotePicture>(ptr);
		let org_result = fn_org(arg);
		log::trace!("GamePictureManager_RequestRemotePicture({arg}) -> {org_result}");
		org_result
	}
}

//TODO: (Consider) Getting profile username from BlurAPI instead?
pub fn get_saved_profile_username_v2(ptr_base: *mut c_void) -> Result<String, Utf8Error> {
	// "Blur.exe"+0xE144E1
	const OFFSET_PROFILE_USERNAME: isize = 0xE144E1;

	let ptr = ptr_base.wrapping_offset(OFFSET_PROFILE_USERNAME) as *const std::ffi::c_char;
	let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
	match s.to_str() {
		Ok(s) => Ok(s.to_string()),
		Err(e) => {
			log::error!("Could not read profile username as UTF-8 &str.");
			Err(e)
		}
	}
}

pub fn get_local_default_pfp_filepath() -> Result<PathBuf, std::io::Error> {
	let dir = known_folders::get_known_folder_path(known_folders::KnownFolder::RoamingAppData)
		.ok_or_else(|| io::Error::other("Couldn't get FOLDERID_RoamingAppData (defaut: %USERPROFILE%\\AppData\\Roaming [%APPDATA%]) from system"))?
		.join("bizarre creations")
		.join("blur")
		.join("amax");
	if !&dir.is_dir() {
		std::fs::create_dir_all(&dir)?;
	};
	let local_pfp_path = dir.join("pfp.bmp");
	Ok(local_pfp_path)
}

/// First try downloading pic from AMAX for that username
/// If that fails, try a downloading a default one
/// If downloading default also fails, try reading something from disk at default location
/// If eighter was downloaded, store it on disk at default location
/// NOTE: >Note for the people in the future:
///   I propose removing all logic dealing with downloading defaults.
///   Just simply always download instead.
///   (Maybe, if downloading fails, use something from disk that the user put there manually. Maybe...!)
fn get_primary_profile_img_data(username: &str) -> Result<Vec<u8>, std::io::Error> {
	let pfp_path = &get_local_default_pfp_filepath()?;
	let pfp_path_display = pfp_path.display();
	match get_amax_user_pfp_img_data(username).or_else(|_| get_default_amax_pfp_img_data()) {
		Ok(img_data) => {
			// For now I think it is ok to just TRY to store it
			// We can still continue if storage fails
			match std::fs::write(pfp_path, &img_data) {
				Ok(()) => {
					log::trace!(
						"Saved downloaded pfp data for \"{username}\" to disk [{pfp_path_display}]"
					);
				}
				Err(e) => {
					log::error!("Error saving downloaded pfp data for \"{username}\" to [{pfp_path_display}]: {e}");
				}
			};
			return Ok(img_data); // YAY!
		}
		Err(e) => {
			log::warn!("Failed to get pfp img_data via http for \"{username}\": {e}");
		}
	};

	// try to read pfp from disk
	match std::fs::read(pfp_path) {
		Ok(img_data) => {
			log::trace!("Got primary profile picture from disk [{pfp_path_display}]");
			Ok(img_data)
		}
		Err(e) => {
			log::error!(
				"Failed to read primary profile picture img_data from disk [{pfp_path_display}]: {e}"
			);
			Err(e)
		}
	}
}

#[allow(unused)]
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
