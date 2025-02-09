use image::{imageops::FilterType::Lanczos3, ImageOutputFormat};
use std::io::Cursor;
use std::io::Read;

#[derive(Debug)]
#[allow(unused)]
pub enum AmaxImgError {
	HttpFailedRequest(String),
	HttpBadResponse,
	BadParsing { e: image::error::ImageError },
}

impl std::fmt::Display for AmaxImgError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "({self:?})")
	}
}
impl std::error::Error for AmaxImgError {}

pub fn get_image_from_url(url: String) -> Result<Vec<u8>, AmaxImgError> {
	const URL_AMAX_LOGO: &str = "https://cs.amax-emu.com/amax_logo.png";
	log::info!("Req: {url}");
	let resp = match ureq::get(&url).call() {
		Ok(resp) => resp,
		Err(e) => {
			log::error!("Failed to HTTP request: {e}");
			log::info!("Req fallback: {URL_AMAX_LOGO}");
			match ureq::get(URL_AMAX_LOGO).call() {
				Ok(resp) => resp,
				Err(e) => {
					log::error!("Failed to HTTP request Fallback: {e}");
					return Err(AmaxImgError::HttpFailedRequest(e.to_string()));
				}
			}
		}
	};

	let len = match resp
		.header("Content-Length")
		.and_then(|s| s.parse::<usize>().ok())
	{
		Some(content_size) => content_size,
		None => {
			log::error!("Response from the server is missing Content-Length header.");
			return Err(AmaxImgError::HttpBadResponse);
		}
	};

	let mut bytes: Vec<u8> = Vec::with_capacity(len);

	resp.into_reader()
		.take(10_000_000)
		.read_to_end(&mut bytes)
		.unwrap();

	let mut img = match image::load_from_memory(&bytes) {
		Ok(img) => img,
		Err(e) => {
			log::error!("Failed to parse downloaded image: {e}");
			return Err(AmaxImgError::BadParsing { e });
		}
	};

	img = img.resize(64, 64, Lanczos3);

	//REMOVE IN RELEASE
	//img = img.huerotate(rand::thread_rng().gen_range(0..360));
	//REMOVE IN RELEASE

	let mut return_vec: Vec<u8> = vec![];
	img.write_to(&mut Cursor::new(&mut return_vec), ImageOutputFormat::Bmp)
		.unwrap();

	//img.save_with_format("./debug.bmp", ImageFormat::Bmp);
	//std::fs::write("./debug2.bmp", &return_vec);

	Ok(return_vec)
}
