use std::ffi::c_void;

use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

use crate::{
	d3d9_utils::create_64x64_d3d9tex, gamer_picture_manager::{
		trigger_lobby_update_v2, C_GamerPicture, GamerPictureManager, NetPlayer,
	}, img_preprocess::{get_amax_user_pfp_img_data, get_default_amax_pfp_img_data}, CoolBlurPlugin
};


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
				let default_player_img_data = get_default_amax_pfp_img_data().unwrap();
				log::info!(
					"Got default img_data ({} bytes) for \"{username}\" via HTTP.",
					default_player_img_data.len()
				);
				default_player_img_data
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

//FIXME: Rust aliasing rules make this kind of &mut T reference unsound (*SOMETIMES!*)
// The compiler can assume that there is nothing else pointing to T, and do stuff we don't want.
// I don't know how likely that is tho.
// I think in most cases here we should be fine.
// It would be better to just return the raw pointer  -> `Option<*mut GamerPictureManager>`
// I hate working the *mut T though, all the unsafe and all the dereferencing with (*T) is annoying.
// Also this whole DLL is unsafe and Blur is unsafe AND WHO IS EVEN GONNA READ THIS LALALAlalalal ITS MY PULL REQUEST I CAN DO WHAT I WANT
pub fn get_gamer_picture_manager_v2<'a>(
	ptr_base: *mut c_void,
) -> Option<&'a mut GamerPictureManager> {
	const ADDR_GPM_OFFSET: isize = 0xDA89C8;
	let p: *mut *mut GamerPictureManager = ptr_base.wrapping_byte_offset(ADDR_GPM_OFFSET) as _;
	unsafe {
		let p: *mut GamerPictureManager = p.read();
		if p.is_null() {
			None
		} else {
			Some(&mut (*p))
		}
	}
}

pub fn run() {
	log::trace!("Hellooooo from the other sideeeeeeee (thread)");
	let mut my_cache: MyCache = MyCache::new();
	let ptr_base: *mut c_void = CoolBlurPlugin::get_exe_base_ptr();
	loop {
		std::thread::sleep(std::time::Duration::from_millis(1000));

		let mut racist = get_first_lobby_net_racer(ptr_base);
		if racist.is_none() {
			log::trace!(
				"No get_first_lobby_net_racer() obtained from Linked List of NetRacists. We are probably not in a lobby."
			);
			continue;
		};

		let Some(gpm) = get_gamer_picture_manager_v2(ptr_base) else {
			// I don't know where this should happen exactly
			log::warn!("No get_gamer_picture_manager()...");
			continue;
		};

		let pics = unsafe { *(gpm.remote_pictures_ptr) };

		// The index in the linked list of NetRacists
		// The local player is always at the first position in the linked list
		let mut racist_idx: usize = 0;
		// The other 19 possible players will be second, third, fourth, ...
		let mut pics_idx: usize = racist_idx;
		let mut lobby_needs_update: bool = false;

		while let Some(p) = racist {
			let name = p.get_username();
			let dwid = p.get_dw_id();
			let refid = p.get_lobby_ref();
			log::info!(
				"Got player NetRacist #{racist_idx} in lobby: \"{name}\" [{dwid}]. Their ref is: {refid}"
			);
			//NOTE: refid is u8 and handled by the game. I have no idea what happens when it reaches 255...
			// a refid=0 usually means that game is still loading lobby data
			// racist_idx = 0 is for local player
			if (0 < refid) && (0 < racist_idx) {
				let pic: &mut C_GamerPicture = unsafe { &mut *(*(pics.get(pics_idx).unwrap())) };
				// TODO: This overwrites (and thus triggers lobby update) on every cycle
				// It might be better to only overwrite when necessary: only when NetPlayer.dwid != pic.dwid
				pic.ref1 = refid as u16;
				pic.user_dw_id = dwid;
				pic.active = true;
				pic.free = false;

				log::info!("Setting tex for NetRacist #{racist_idx} \"{name}\" ref:{refid} in remote_pictures[{pics_idx}]");
				//NOTE: MyCache::get(..) could take a while.
				// It is possible that the lobby info changes in the meantime (players joining and leaving, local disconnect)
				// If it desyncs, I think that gets resolved in a few cycles
				// Ideal would be to handle the downloading to cache in another thread
				// Then here we only set texture IF it has already been obtained...
				//TODO: thread for downloading
				pic.texture_ptr = my_cache.get(&name);
				pics_idx += 1;

				lobby_needs_update = true;
			}
			racist_idx += 1;
			racist = p.get_next();
		}
		// Clear data for players that left the lobby
		// They didn't show up the NetRacers linked list, so their data in Pics should be cleared
		for remaining_pic_idx in pics_idx..pics.len() {
			let pic: &mut C_GamerPicture =
				unsafe { &mut *(*(pics.get(remaining_pic_idx).unwrap())) };
			if 0 < pic.ref1 {
				log::trace!("Clearing C_GamerPicture data @ GamerPictureManager.remote_pictures[{remaining_pic_idx}]");
				lobby_needs_update = true;
			}
			pic.ref1 = 0u16; // still do it anyway :D just in case...
			pic.user_dw_id = 0u64;
			pic.active = false;
			pic.free = true;
		}
		if lobby_needs_update {
			trigger_lobby_update_v2(ptr_base);
		}
	}
}
