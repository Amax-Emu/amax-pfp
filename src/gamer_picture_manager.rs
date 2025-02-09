use crate::img_preprocess::{get_image_from_url, AmaxImgError};

use fxhash::FxHashMap;
use retour::static_detour;
use std::ffi::c_void;
use std::time::Duration;
use std::{io, mem, str::Utf8Error};
use widestring::WideCString;
use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

/// Due to how memory in Blur works there are some static locations in memory, that contain pointers to some structures.
/// This one points to GAMER_PICTURE_MANAGER, which is great.
pub static GAMER_PICTURE_MANAGER: i32 = 0x011a89c8;

/// This one points to your friend list.
pub static _FRIEND_LIST: i32 = 0x011C7040;

//static URL_BASE: String = String::from("https://amax-emu.com/api");

//static PFP_CACHE: HashMap<String, Vec<u8>> = Lazy::new(||HashMap::new());

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
	pub ref1: u16,
	pub user_dw_id: u64,
	user_information: [u8; 8], // 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00
	pub active: bool,          // 0x00
	pub free: bool,            // 0x01
	pub gamer_pic_name: [u8; 30], //GAMERPIC_X or REMOTE_GAMERPIC_X
	size_as_big_end_temp: u32, // 0x00, 0x00, 0x00, 0x00
	unk_zeroes: u32,           // 0x00, 0x40 0x00, 0x00,
	unk_4_as_u16: u16,         //0x04, 0x00,
	pub texture_ptr: IDirect3DTexture9, //0xE0, 0x71 0x90, 0x14
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
			return None;
		}
		return Some(unsafe { &mut *self.ptr_to_next });
	}
	pub fn get_dw_id(&self) -> u64 {
		self.user_dw_id
	}
	pub fn get_username(&self) -> String {
		WideCString::from_vec_truncate(&self.username_in_utf_16)
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

#[derive(Debug)]
#[repr(C)]
#[allow(unused)]
pub struct MpUiLobbyData {
	// A rather short, but very usefull struct.
	unk0: [u8; 28],
	net_players: *mut [NetPlayer; 19],
	total_players: u32, //client itself also counts, so to get number of connected remote users substract 1
}

static_detour! {
	static GetPrimaryProfilePictureHook: unsafe extern "system" fn() -> bool;
}

static_detour! {
	static GamePictureManager_CreateHook: unsafe extern "system" fn(i32,i32,*const [u8;32],bool) -> bool;
}

static_detour! {
	static GamePictureManager_RequestRemotePicture: unsafe extern "system" fn(i32) -> bool;
}

//0079da10
static_detour! {
	/// little pesky function messing up things
	static GamePictureManager_WipeRemotePictures: unsafe extern "fastcall" fn(*mut GamerPictureManager);
}

pub unsafe fn create_get_primary_profile_picture_hook() {
	const ORG_FN_ADDRESS: isize = 0x00d5e170;
	type FnCreatePrimaryProfilePicture = unsafe extern "system" fn() -> bool;
	let target = mem::transmute::<isize, FnCreatePrimaryProfilePicture>(ORG_FN_ADDRESS);
	GetPrimaryProfilePictureHook
		.initialize(target, primary_picture_load)
		.unwrap()
		.enable()
		.unwrap();
}

unsafe fn get_gamer_picture_manager() -> GamerPictureManager {
	let local_start = GAMER_PICTURE_MANAGER;
	log::debug!("Addr of start: {:?}", local_start);

	let ptr = local_start as *const i32;
	let ptr = *ptr as *mut GamerPictureManager;
	log::debug!("Addr of GamerPictureManager ptr: {:p}", &ptr);
	//todo: there could be cases were GPM wouldn't be initiated.
	*ptr
}

pub fn pretty_name(name_buf: &[u8]) -> String {
	let name = String::from_utf8(name_buf.to_vec()).unwrap();
	name.trim_matches(char::from(0)).to_string()
}

fn primary_picture_load() -> bool {
	log::info!("GetPrimaryProfilePicture hook");
	unsafe {
		let gamer_picture_manager = get_gamer_picture_manager();
		let local_picures = gamer_picture_manager.local_pictures_ptr.read();

		for picture_ptr in local_picures {
			let name = pretty_name(&(*picture_ptr).gamer_pic_name);

			if name == "GAMERPIC_0" {
				let username = match get_saved_profile_username() {
					Ok(username) => username.to_string(),
					Err(e) => {
						log::error!(
							"Failed to get_saved_profile_username: {e}. Skipping pfp setup."
						);
						continue;
					}
				};
				log::info!("Loading primary picture for \"{username}\"");

				//let img_data = get_image_from_url("https://cdn.discordapp.com/avatars/925665499692544040/483eb1b92db6a449a0e2bed9a8b48bb3.png");
				let mut img_data = match get_primary_profile_pic(&username) {
					Ok(img_data) => img_data,
					Err(e) => {
						log::error!("Failed to get local image {e}");
						continue;
					}
				};

				let d3d9_monke: *mut IDirect3DTexture9 =
					std::ptr::addr_of_mut!((*picture_ptr).texture_ptr);
				crate::d3d9_utils::d3d9_create_tex_from_mem_ex(d3d9_monke, &mut img_data, 64, 64)
					.unwrap();
				(*picture_ptr).active = true;
				(*picture_ptr).free = false;
				log::info!("We did the d3d9_monke: {d3d9_monke:?}");
			}
		}
	}
	false
}

//Yes, we gonna copy-paste same code for retreiving username from project to project, why are you asking?
pub fn get_saved_profile_username() -> Result<String, Utf8Error> {
	use std::ffi::{c_char, CStr};
	use windows::{core::PCSTR, Win32::System::LibraryLoader::GetModuleHandleA};

	let ptr_base: *mut std::ffi::c_void =
		unsafe { GetModuleHandleA(PCSTR::null()) }.unwrap().0 as _;

	// "Blur.exe"+0xE144E1
	const OFFSET_PROFILE_USERNAME: isize = 0xE144E1;

	let ptr = ptr_base.wrapping_offset(OFFSET_PROFILE_USERNAME) as *const c_char;
	let s = unsafe { CStr::from_ptr(ptr) };
	match s.to_str() {
		Ok(str) => Ok(str.to_string()),
		Err(e) => {
			log::error!("Could not read username as UTF-8 str from profile.");
			Err(e)
		}
	}
}

fn get_primary_profile_pic(username: &str) -> Result<Vec<u8>, std::io::Error> {
	//first we try to get picture though HTTP

	let dir = known_folders::get_known_folder_path(known_folders::KnownFolder::RoamingAppData)
		.ok_or_else(|| io::Error::other("Couldn't get FOLDERID_RoamingAppData (defautl: %USERPROFILE%\\AppData\\Roaming)] as a KnownFolder"))
		.unwrap()
		.join("bizarre creations")
		.join("blur")
		.join("amax");

	if !&dir.is_dir() {
		let dir_display = dir.display();
		std::fs::create_dir_all(&dir)
			.unwrap_or_else(|_| panic!("Failed to create amax folder in AppData: {dir_display}"));
	};

	let local_pfp_path = &dir.join("./pfp.bmp");
	let local_pfp_path_display = local_pfp_path.display();

	log::info!("Req: {username} -> {local_pfp_path_display}");
	match get_pfp_via_http_for_username(username) {
		Ok(img_data) => {
			std::fs::write(local_pfp_path, &img_data).unwrap();
			log::info!("Saved to {local_pfp_path_display}");
			return Ok(img_data); // YAY!
		}
		Err(e) => {
			log::error!("Failed to get image via http: {e:#?}");
		}
	};

	// attempt to fetch it from local cache, located in %APPDATA%
	match std::fs::read(local_pfp_path) {
		Ok(img_data) => {
			log::info!("Read from {local_pfp_path_display}");
			Ok(img_data)
		}
		Err(e) => {
			log::error!("Failed to read ima_data from {local_pfp_path_display}: {e}");
			Err(e)
		}
	}
}

pub fn get_pfp_via_http_for_username(username: &str) -> Result<Vec<u8>, AmaxImgError> {
	//"https://cdn.discordapp.com/avatars/925665499692544040/483eb1b92db6a449a0e2bed9a8b48bb3.png"
	//"https://cs.amax-emu.com/amax_logo.png"
	get_image_from_url(std::format!(
		"https://amax-emu.com/api/players/pfp/name/{username}"
	))
}

pub unsafe fn remote_pfp_updater(ptr_base: *mut c_void) {
	let mut pfp_cache: FxHashMap<u64, Vec<u8>> = FxHashMap::default();

	loop {
		std::thread::sleep(Duration::from_secs(1));

		let ptr: *mut *mut MpUiLobbyData = ptr_base.wrapping_byte_offset(0x00DB4530) as _;
		let ptr_ptr: *mut MpUiLobbyData = ptr.read();
		if ptr_ptr.is_null() {
			log::warn!("mp_ui_lobby_data pointer empty...");
			continue;
		}
		let mp_ui_lobby_data: MpUiLobbyData = ptr_ptr.read();

		let player_count = mp_ui_lobby_data.total_players;

		if player_count < 2 {
			log::debug!("We're alone in the lobby ({player_count})...");
			continue;
		}

		log::debug!("We're not alone (MpUiLobbyData.total_players={player_count}).");

		let gamer_picture_manager = get_gamer_picture_manager();
		log::debug!("Got gamer_picture_manager");
		let remote_pictures = (gamer_picture_manager.remote_pictures_ptr).read();
		log::debug!("Got Remote pictures from gamer_picture_manager");

		/* for (idx, remote_pic) in remote_pictures
			.iter()
			.enumerate()
			.filter(|(_idx, pic)| !pic.is_null())
			.map(|(idx, pic)| (idx, pic.read()))
		{
			// log::debug!("remote_pictures[{idx}]: {remote_pic:?}");
		} */
		let mp_ui_lobby_data_net_racers: [NetPlayer; 19] = (mp_ui_lobby_data.net_players).read();

		/* for player in mp_ui_lobby_data_net_racers {
			// dbg!(player);
		} */
		log::debug!("Got network players from MpUiLobbyData.net_players");

		for player_idx in 0..(mp_ui_lobby_data.total_players as usize - 1) {
			let net_player: NetPlayer = mp_ui_lobby_data_net_racers[player_idx];
			let net_player_name = WideCString::from_vec_truncate(net_player.username_in_utf_16)
				.to_string()
				.unwrap();
			log::debug!(
				"mp_ui_lobby_data_net_racers[{player_idx}].username_in_utf_16={net_player_name}"
			);

			let remote_gamerpic_data_ptr: *mut C_GamerPicture = remote_pictures[player_idx];

			let net_player_dw_id = net_player.user_dw_id;
			if (*remote_gamerpic_data_ptr).user_dw_id == net_player_dw_id {
				log::debug!("PFP for \"{net_player_name}\" [dw:{net_player_dw_id}] MpUiLobbyData[{player_idx}] is already set.");
				if !(*remote_gamerpic_data_ptr).free {
					//failsafe
					(*remote_gamerpic_data_ptr).ref1 = net_player.mp_lobby_ref_id as u16;
				}
				continue;
			}

			let mut img_data = match pfp_cache.get(&net_player_dw_id) {
				Some(data) => {
					log::info!("Got pfp img_data for {net_player_name} via pfp_cache");
					data.clone()
				}
				None => match get_pfp_via_http_for_username(&net_player_name) {
					Ok(data) => {
						log::info!("Got pfp img_data for {net_player_name} via http");
						pfp_cache.insert(net_player_dw_id, data.clone());
						data
					}
					Err(e) => {
						log::error!("Failed to retrive image for {net_player_name}: {e}");
						continue;
					}
				},
			};

			let remote_gamerpic_d3d9tex_ptr: *mut IDirect3DTexture9 =
				std::ptr::addr_of_mut!((*remote_gamerpic_data_ptr).texture_ptr);
			crate::d3d9_utils::d3d9_create_tex_from_mem_ex(
				remote_gamerpic_d3d9tex_ptr,
				&mut img_data,
				64,
				64,
			)
			.unwrap();

			log::info!("set remote_gamerpic_data monke stuff: {remote_gamerpic_d3d9tex_ptr:?}");
			(*remote_gamerpic_data_ptr).user_dw_id = net_player.user_dw_id;
			(*remote_gamerpic_data_ptr).ref1 = net_player.mp_lobby_ref_id as u16;
			(*remote_gamerpic_data_ptr).active = true;
			(*remote_gamerpic_data_ptr).free = false;
			let _ = trigger_lobby_update(ptr_base);
			log::info!("DONE for {net_player_name}");
		}
	}
}

#[allow(unused)]
pub unsafe fn trigger_lobby_update(ptr_base: *mut c_void) -> Result<(), ()> {
	//the final piece of the puzzle
	log::debug!("Triggering lobby update");
	let start = ptr_base.wrapping_offset(0x00E42FF8);

	let ptr = start as *const i32;
	log::debug!("Addr of ptr1: {:p},value: 0x0{:X}", ptr, *ptr);

	if *ptr == 0 {
		return Err(());
	};

	let step2 = *ptr;

	let step3 = step2 + 0x181;

	let lobby_need_update = step3 as *mut bool;
	*lobby_need_update = true;
	Ok(())
}

#[allow(unused)]
pub fn vv_trigger_lobby_update(ptr_base: *mut c_void) {
	let p: *mut *mut bool = ptr_base.wrapping_offset(0x00E42FF8) as _;
	unsafe {
		let p: *mut bool = p.read();
		if p.is_null() {
			log::warn!("failed to vv_trigger_lobby_update()");
			return;
		} else {
			log::trace!("w:vv_trigger_lobby_update()");
			p.wrapping_byte_offset(0x181).write(true);
		}
	}
}
