use std::ffi::c_void;

use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

use crate::gamer_picture_manager::{
	get_pfp_via_http_for_username, pretty_name, vv_trigger_lobby_update, C_GamerPicture,
	GamerPictureManager, MpUiLobbyData, NetPlayer,
};

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

fn get_crusty_d3d9_tex(p: *mut IDirect3DTexture9, crusty_img_data: &mut Vec<u8>) {
	crate::d3d9_utils::d3d9_create_tex_from_mem_ex(p, crusty_img_data, 64, 64).unwrap();
}

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

fn get_mp_ui_lobby_data<'a>(ptr_base: *mut c_void) -> Option<&'a MpUiLobbyData> {
	let p: *mut *mut MpUiLobbyData = ptr_base.wrapping_byte_offset(0x00DB4530) as _;
	unsafe {
		let p: *mut MpUiLobbyData = p.read();
		if p.is_null() {
			None
		} else {
			Some(&(*p))
		}
	}
}

struct MyGamerData {
	pub name: String,
	pub img_data: Vec<u8>,
	pub tex_ptr: *mut IDirect3DTexture9,
}

impl MyGamerData {
	pub fn new(username: &str, ptr: *mut IDirect3DTexture9) -> Self {
		let mut user_img_data =
			get_pfp_via_http_for_username(username).unwrap_or_else(|_| get_crusty_img_data());
		//get_crusty_img_data();
		log::info!("GET DATA [{username}]: {}", user_img_data.len());
		get_crusty_d3d9_tex(ptr, &mut user_img_data);
		Self {
			name: username.to_string(),
			img_data: user_img_data,
			tex_ptr: ptr,
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

	pub fn get(&mut self, username: &str, ptr: *mut IDirect3DTexture9) {
		match self.gamers.iter_mut().find(|gamer| gamer.name == username) {
			Some(gamer) => {
				log::info!("found \"{username}\" in pfp cache!");
				unsafe { ptr.write(gamer.tex_ptr.read()) };
			}
			None => {
				log::info!("adding \"{username}\" to pfp cache!");
				self.gamers.push(MyGamerData::new(username, ptr));
			}
		};
	}
}

fn get_gamer_picture_manager<'a>(ptr_base: *mut c_void) -> Option<&'a GamerPictureManager> {
	// const GAMER_PICTURE_MANAGER_1: isize = 0x011a89c8;
	const ADDR_GPM: isize = 0xDA89C8;
	let p: *mut *mut GamerPictureManager = ptr_base.wrapping_byte_offset(ADDR_GPM) as _;
	unsafe {
		let p: *mut GamerPictureManager = p.read();
		if p.is_null() {
			None
		} else {
			Some(&(*p))
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

		let Some(gpm) = get_gamer_picture_manager(ptr_base) else {
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
		{
			for (idx, pic) in pics.into_iter().map(|p| (unsafe { &*p })).enumerate() {
				if true {
					continue;
				}
				log::info!(
					"
C_GamerPicture[{idx}] (
	ref1: u16 = {},
	user_dw_id: u64 = {},
	active: bool = {},
	free: bool = {},
	gamer_pic_name: [u8; 30] = {},
	texture_ptr: IDirect3DTexture9 = {:?},
	default_texture_ptr: u32 = {}
)
",
					pic.ref1,
					pic.user_dw_id,
					pic.active,
					pic.free,
					pretty_name(&pic.gamer_pic_name),
					pic.texture_ptr,
					pic.default_texture_ptr,
				);
			}
		}
		let mut racist_idx: usize = 0;
		let mut lobby_needs_update: bool = false;
		while let Some(p) = racist {
			let name = p.get_username();
			let dwid = p.get_dw_id();
			let refid = p.get_lobby_ref();
			log::info!(
				"Got player #{racist_idx} in lobby: \"{name}\" [{dwid}]. Their ref is: {refid}"
			);
			//NOTE: How should the game handle refid reaching 255?
			if (0 < refid) && (0 < racist_idx) {
				let pic: &mut C_GamerPicture = unsafe { &mut *(*(pics.get(racist_idx).unwrap())) };
				pic.ref1 = refid as u16;
				pic.user_dw_id = dwid;
				pic.active = true;
				pic.free = false;
				log::info!("Setting crusty tex for #{racist_idx} \"{name}\" ref:{refid}");
				{
					let remote_gamerpic_d3d9tex_ptr = std::ptr::addr_of_mut!(pic.texture_ptr);
					my_cache.get(&name, remote_gamerpic_d3d9tex_ptr);
					/*
					crate::d3d9_utils::d3d9_create_tex_from_mem_ex(
						remote_gamerpic_d3d9tex_ptr,
						&mut img_data,
						64,
						64,
					)
					.unwrap();
					*/
				}
				//
				lobby_needs_update = true;
			}
			racist_idx += 1;
			racist = p.get_next();
		}
		if lobby_needs_update {
			vv_trigger_lobby_update(ptr_base);
		}
	}
}
