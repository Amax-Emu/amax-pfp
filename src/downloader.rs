use image::{imageops::FilterType::Lanczos3, ImageOutputFormat};
use std::{io::Read, path::PathBuf};

#[derive(Debug)]
#[allow(unused)]
pub enum AmaxImgError {
	HttpFailedRequest(String),
	HttpBadResponse,
	HttpBadContentLength,
	BadParsing { e: image::error::ImageError },
}

impl std::fmt::Display for AmaxImgError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "({self:?})")
	}
}
impl std::error::Error for AmaxImgError {}

fn get_local_default_pfp_filepath() -> Result<PathBuf, std::io::Error> {
	let dir = known_folders::get_known_folder_path(known_folders::KnownFolder::RoamingAppData)
		.ok_or_else(|| std::io::Error::other("Couldn't get FOLDERID_RoamingAppData (defaut: %USERPROFILE%\\AppData\\Roaming [%APPDATA%]) from system"))?
		.join("bizarre creations")
		.join("blur")
		.join("amax");
	if !&dir.is_dir() {
		std::fs::create_dir_all(&dir)?;
	};
	let local_pfp_path = dir.join("pfp.bmp");
	Ok(local_pfp_path)
}

pub fn get_default_amax_pfp_img_data() -> Result<Vec<u8>, AmaxImgError> {
	//"https://cdn.discordapp.com/avatars/925665499692544040/483eb1b92db6a449a0e2bed9a8b48bb3.png"
	const URL_AMAX_LOGO: &str = "https://cs.amax-emu.com/amax_logo.png";
	get_image_from_url(URL_AMAX_LOGO)
}

pub fn get_amax_user_pfp_img_data(username: &str) -> Result<Vec<u8>, AmaxImgError> {
	get_image_from_url(&std::format!(
		"https://amax-emu.com/api/players/pfp/name/{username}"
	))
}

fn get_image_from_url(url: &str) -> Result<Vec<u8>, AmaxImgError> {
	let response = ureq::get(url).call().map_err(|e| {
		log::error!("Failed HTTP request: {e}");
		AmaxImgError::HttpFailedRequest(e.to_string())
	})?;

	let len = match response
		.header("Content-Length")
		.and_then(|s| s.parse::<usize>().ok())
	{
		Some(content_size) => content_size,
		None => {
			log::error!("Response from the server is missing Content-Length header.");
			return Err(AmaxImgError::HttpBadResponse);
		}
	};

	//TODO: Is this the best way? Check ureq::response::Response
	const MAX_READ_LIMIT: u64 = 8 * 1024 * 1024; // 8 MB?
	let mut bytes: Vec<u8> = Vec::with_capacity(len);
	let recv_read_len = response
		.into_reader()
		.take(MAX_READ_LIMIT)
		.read_to_end(&mut bytes)
		.expect("Response from server is too long to read_to_end(..)"); // idk what this is
	log::trace!("Recv back server response of {recv_read_len} bytes.");

	let img = image::load_from_memory(&bytes).map_err(|e| {
		log::error!("Failed read img_data downloaded from [{url}]: {e}");
		AmaxImgError::BadParsing { e }
	})?;

	let img = img.resize(64, 64, Lanczos3);

	let mut return_vec: Vec<u8> = vec![];
	img.write_to(
		&mut std::io::Cursor::new(&mut return_vec),
		ImageOutputFormat::Bmp,
	)
	.map_err(|e| {
		log::error!(
			"Failed to transform downloaded image from [{url}] into ImageOutputFormat::Bmp: {e}"
		);
		AmaxImgError::BadParsing { e }
	})?;

	Ok(return_vec) // YAY!
}

/// First try downloading pic from AMAX for that username
/// If that fails, try a downloading a default one
/// If downloading default also fails, try reading something from disk at default location
/// If eighter was downloaded, store it on disk at default location
/// NOTE: >Note for the people in the future:
///   I propose removing all logic dealing with downloading defaults.
///   Just simply always download instead.
///   (Maybe, if downloading fails, use something from disk that the user put there manually. Maybe...!)
pub fn get_primary_profile_img_data(username: &str) -> Result<Vec<u8>, std::io::Error> {
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
