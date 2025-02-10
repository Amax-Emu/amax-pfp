use image::{imageops::FilterType::Lanczos3, ImageOutputFormat};
use std::io::Read;

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

	/// WTF
	//TODO: Is this the best way? Check ureq::response::Response
	const MAX_READ_LIMIT: u64 = 8 * 1024 * 1024;
	let mut bytes: Vec<u8> = Vec::with_capacity(len);
	let recv_read_len = response
		.into_reader()
		.take(MAX_READ_LIMIT)
		.read_to_end(&mut bytes)
		.expect("recv too many bytes to read!"); // idk what this is
	log::trace!("Got back {recv_read_len} bytes in ureq::response::Response");

	let mut img = image::load_from_memory(&bytes).map_err(|e| {
		log::error!("Failed read img_data downloaded from [{url}]: {e}");
		AmaxImgError::BadParsing { e }
	})?;

	img = img.resize(64, 64, Lanczos3);

	//img = img.huerotate(rand::thread_rng().gen_range(0..360)); // :>

	let mut return_vec: Vec<u8> = vec![];
	img.write_to(
		&mut std::io::Cursor::new(&mut return_vec),
		ImageOutputFormat::Bmp,
	)
	.map_err(|e| {
		log::error!(
			"Failed transform img_data downloaded from [{url}] to ImageOutputFormat::Bmp: {e}"
		);
		AmaxImgError::BadParsing { e }
	})?;

	Ok(return_vec) // YAY!
}
