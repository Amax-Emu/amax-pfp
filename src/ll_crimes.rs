use std::ffi::c_void;

use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

use crate::{
	gamer_picture_manager::{
		trigger_lobby_update_v2, C_GamerPicture, GamerPictureManager, NetPlayer,
	},
	img_preprocess::get_amax_user_pfp_img_data,
};

//FIXME: Remove this
fn get_crusty_img_data() -> Vec<u8> {
	let dir = known_folders::get_known_folder_path(known_folders::KnownFolder::RoamingAppData)
		.unwrap()
		.join("bizarre creations")
		.join("blur")
		.join("amax")
		.join("test.bmp");
	log::warn!("Gettin crusty from {}", dir.display());
	std::fs::read(dir).unwrap()
}

pub fn create_64x64_d3d9tex(img_data: &mut [u8]) -> *mut IDirect3DTexture9 {
	crate::d3d9_utils::d3d9_create_tex_from_mem_ex(img_data, 64, 64)
}

// Gorgeous Precious Beautiful Majestic
// (Okay maybe not Majestic)
// There is a Linked List of NetRacers
// This is how you can obtain the first NetPlayer in the list
// NOTE: This might be the first time I find an actual Linked List that is somewhat useful in the wild!
fn get_first_lobby_net_racer<'a>(ptr_base: *mut c_void) -> Option<&'a mut NetPlayer> {
	let p: *mut *mut *mut NetPlayer = ptr_base.wrapping_byte_offset(0xDB4530) as _;
	// * -> [[[Blur.exe + 0xDB4530] + 0x18] + 0]
	unsafe {
		let p: *mut NetPlayer = p.read().wrapping_byte_offset(0x18).read();
		if p.is_null() {
			return None;
		}
		Some(&mut (*p))
	}
}

#[allow(unused)]
struct MyGamerData {
	pub name: String,
	pub img_data: Vec<u8>,
	pub tex_ptr: *mut IDirect3DTexture9,
}

impl MyGamerData {
	pub fn new(username: &str) -> Self {
		let mut user_img_data = get_amax_user_pfp_img_data(username)
			.inspect(|http_img_data| {
				log::info!(
					"Got img_data ({} bytes) for \"{username}\" via HTTP.",
					http_img_data.len()
				)
			})
			.unwrap_or_else(|_| {
				let crusty_img_data = get_crusty_img_data();
				log::info!(
					"Got img_data ({} bytes) for \"{username}\" via disk crusty.",
					crusty_img_data.len()
				);
				crusty_img_data
			});
		let tex = create_64x64_d3d9tex(&mut user_img_data);
		Self {
			name: username.to_string(),
			img_data: user_img_data,
			tex_ptr: tex,
		}
	}
}

struct MyCache {
	pub gamers: Vec<MyGamerData>,
}

impl MyCache {
	pub fn new() -> Self {
		Self { gamers: vec![] }
	}

	pub fn get(&mut self, username: &str) -> *mut IDirect3DTexture9 {
		let gamer = match self.gamers.iter_mut().find(|gamer| gamer.name == username) {
			Some(gamer) => {
				log::info!("found \"{username}\" in pfp cache!");
				gamer
			}
			None => {
				log::info!("adding \"{username}\" to pfp cache!");
				self.gamers.push(MyGamerData::new(username));
				self.gamers.last_mut().unwrap()
			}
		};
		gamer.tex_ptr
	}
}

pub fn get_gamer_picture_manager_v2<'a>(
	ptr_base: *mut c_void,
) -> Option<&'a mut GamerPictureManager> {
	/// Due to how memory in Blur works there are some static locations in memory, that contain pointers to some structures.
	/// This one points to GAMER_PICTURE_MANAGER, which is great.
	// const GAMER_PICTURE_MANAGER_1: isize = 0x011a89c8; // 0x11a89c8 is without ptr_base offset
	const ADDR_GPM: isize = 0xDA89C8;
	let p: *mut *mut GamerPictureManager = ptr_base.wrapping_byte_offset(ADDR_GPM) as _;
	unsafe {
		let p: *mut GamerPictureManager = p.read();
		if p.is_null() {
			None
		} else {
			Some(&mut (*p))
		}
	}
}

pub fn run(ptr_base: *mut c_void) {
	log::info!("Hello from very annoying thread!");
	let mut my_cache: MyCache = MyCache::new();
	loop {
		std::thread::sleep(std::time::Duration::from_millis(1000));

		/*
		let Some(_ui_lobby) = get_mp_ui_lobby_data(ptr_base) else {
			log::warn!("No get_mp_ui_lobby_data()...");
			continue;
		};
		*/

		let mut racist = get_first_lobby_net_racer(ptr_base);
		if racist.is_none() {
			log::trace!("No get_first_lobby_net_racer(). Fails to obtain first racist from LL.");
			continue;
		};

		let Some(gpm) = get_gamer_picture_manager_v2(ptr_base) else {
			log::warn!("No get_gamer_picture_manager()...");
			continue;
		};

		{
			let l_len = gpm.local_pictures_len;
			let l_size = gpm.local_pictures_size;
			let r_len = gpm.remote_pictures_len;
			let r_size = gpm.remote_pictures_size;
			log::trace!("GPM: Local({l_len}/{l_size}) Remote({r_len}/{r_size})");
		}

		let pics = unsafe { *(gpm.remote_pictures_ptr) };

		// The index in the linked list
		// The local player is always at 0
		let mut racist_idx: usize = 0;
		let mut pics_idx: usize = racist_idx;
		let mut lobby_needs_update: bool = false;

		while let Some(p) = racist {
			let name = p.get_username();
			let dwid = p.get_dw_id();
			let refid = p.get_lobby_ref();
			log::info!(
				"Got player #{racist_idx} in lobby: \"{name}\" [{dwid}]. Their ref is: {refid}"
			);
			//NOTE: How should the game handle refid reaching 255?
			// refid = 0 usually means that game is still loading lobby data
			// racist_idx = 0 is for local player
			if (0 < refid) && (0 < racist_idx) {
				let pic: &mut C_GamerPicture = unsafe { &mut *(*(pics.get(pics_idx).unwrap())) };
				pic.ref1 = refid as u16;
				pic.user_dw_id = dwid;
				pic.active = true;
				pic.free = false;

				log::info!("Setting crusty tex for #{racist_idx} \"{name}\" ref:{refid}");
				pic.texture_ptr = my_cache.get(&name);
				pics_idx += 1;

				lobby_needs_update = true;
			}
			racist_idx += 1;
			racist = p.get_next();
		}
		// Clear data for players that left the lobby
		// They didn't show up the NetRacers linked list, so their data in Pics should be cleared
		for remaining_idx in pics_idx..pics.len() {
			let pic: &mut C_GamerPicture = unsafe { &mut *(*(pics.get(remaining_idx).unwrap())) };
			if 0 < pic.ref1 {
				log::trace!("Clearing C_GamerPicture data @ GamerPictureManager.remote_pictures[{remaining_idx}]");
				lobby_needs_update = true;
			}
			pic.ref1 = 0u16; // :> still do it anyway! idk just in case...
			pic.user_dw_id = 0u64;
			pic.active = false;
			pic.free = true;
		}
		if lobby_needs_update {
			trigger_lobby_update_v2(ptr_base);
		}
	}
}
