use std::ffi::c_void;

use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

use crate::{
	data::gamer_picture_manager::{C_GamerPicture, GamerPictureManager},
	data::net_racer::NetPlayer,
	downloader::{get_amax_user_pfp_img_data, get_default_amax_pfp_img_data},
	hooks::trigger_lobby_update_v2,
	tex::create_64x64_d3d9tex,
	MyPlugin,
};

//NOTE: Maybe this should be an enum with two states:
// "Downloading" when the data is still being downloaded
// "Downloaded" when there is data
// "Empty" when there was 404, so we set a default
struct CacheEntry {
	pub name: String,
	pub tex_ptr: *mut IDirect3DTexture9,
}

impl CacheEntry {
	//TODO: Should return Option<Self> if 404 but WHATEVERRRRRRRR
	pub fn new(username: &str) -> Self {
		let mut user_img_data = get_amax_user_pfp_img_data(username)
			.inspect(|http_img_data| {
				log::info!(
					"Downloaded image data ({} bytes) for \"{username}\" via HTTP.",
					http_img_data.len()
				)
			})
			.unwrap_or_else(|_| {
				let default_player_img_data = get_default_amax_pfp_img_data()
					.expect("Failed to download default AMAX profile picture.");
				log::info!(
					"Downloaded default image data ({} bytes) for \"{username}\" via HTTP.",
					default_player_img_data.len()
				);
				default_player_img_data
			});
		let tex = create_64x64_d3d9tex(&mut user_img_data);
		Self {
			name: username.to_string(),
			tex_ptr: tex,
		}
	}
}

//TODO: Store default "https://cs.amax-emu.com/amax_logo.png" somewhere, because we don't need to download it again for every 404...
pub struct Updater {
	// Could also be a HashMap
	gamers: Vec<CacheEntry>,
}

impl Updater {
	fn new() -> Self {
		// new() with default if download 404
		Self { gamers: vec![] }
	}

	fn get(&mut self, username: &str) -> *mut IDirect3DTexture9 {
		let gamer = match self.gamers.iter_mut().find(|gamer| gamer.name == username) {
			Some(gamer) => {
				log::info!("Found \"{username}\" in cache.");
				gamer
			}
			None => {
				self.gamers.push(CacheEntry::new(username));
				log::info!("Adding \"{username}\" to cache.");
				self.gamers.last_mut().unwrap()
			}
		};
		gamer.tex_ptr
	}

	/// Loop responsible for checking the NetRacers Linked List every 1s.
	/// For every NetRacer in the linked list, it fetches their pic, makes it a texture, and writes it into the GamerPictureManager.
	/// It is also responsible for clearing GamerPictureManager slots that don't have a NetRacer associated anymore.
	/// If it changed any data in GamerPictureManager, it triggers a lobby update.
	pub fn run() {
		log::trace!("Updater thread started.");
		let mut my_cache: Updater = Updater::new();
		let ptr_base: *mut c_void = MyPlugin::get_exe_base_ptr();
		loop {
			std::thread::sleep(std::time::Duration::from_millis(1000));

			let mut racist = NetPlayer::get_first_lobby_net_racer(ptr_base);
			if racist.is_none() {
				// log::trace!("No get_first_lobby_net_racer() obtained from Linked List of NetRacists. We are probably not in a lobby.");
				continue;
			};

			let Some(gpm) = GamerPictureManager::summon(ptr_base) else {
				// I don't know where/when/what situaions this would happen exactly...
				log::info!("Couldn't obtain GamerPictureManager, waiting...");
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
				// log::info!("Got player NetRacist #{racist_idx} in lobby: \"{name}\" [{dwid}]. Their ref is: {refid}");
				//NOTE: refid is u8 and handled by the game. I have no idea what happens when it reaches 255...
				// a refid=0 usually means that game is still loading lobby data
				// racist_idx = 0 is for local player
				if (0 < refid) && (0 < racist_idx) {
					let pic: &mut C_GamerPicture =
						unsafe { &mut *(*(pics.get(pics_idx).unwrap())) };
					if pic.user_dw_id != dwid {
						log::trace!("Setting tex for NetRacist #{racist_idx} \"{name}\" ref:{refid} in remote_pictures[{pics_idx}]");
						//NOTE: MyCache::get(..) could take a while.
						// It is possible that the lobby info changes while downloading (players joining and leaving, local disconnect)
						// If it desyncs, I think that gets resolved in a few cycles
						// Ideal would be to handle the downloading to cache in another thread
						// Then here we only set texture IF it has already been obtained...
						//TODO: thread for downloading
						pic.texture_ptr = my_cache.get(&name);

						// TODO: This overwrites (and thus triggers lobby update) on every cycle
						// It might be better to only overwrite when necessary: only when NetPlayer.dwid != pic.dwid
						pic.ref1 = refid as u16;
						pic.user_dw_id = dwid;
						pic.active = true;
						pic.free = false;
						lobby_needs_update = true;
					}
					pics_idx += 1;
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
}
