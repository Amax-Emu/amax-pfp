use std::io::Read;
use image::{GenericImage, GenericImageView, ImageBuffer, RgbImage, imageops::FilterType::Lanczos3, codecs::bmp, ImageOutputFormat};
use std::io::Cursor;

//static URL_BASE: String = String::From("https://amax-emu.com/api/");

pub fn get_image_from_url() -> Vec<u8> {
                    //IMAGE DOWNLOAD
                let resp = ureq::get("https://cdn.discordapp.com/avatars/925665499692544040/483eb1b92db6a449a0e2bed9a8b48bb3.png")
                .call().unwrap();

                let len = resp
                    .header("Content-Length")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap();

                let mut bytes: Vec<u8> = Vec::with_capacity(len);

                resp.into_reader()
                    .take(10_000_000)
                    .read_to_end(&mut bytes)
                    .unwrap();

                let mut img = image::load_from_memory(&bytes).unwrap();

                img = img.resize(64, 64, Lanczos3);

                let mut img_buffer: Cursor<Vec<u8>> = Cursor::new(vec![]);

                img.write_to(&mut img_buffer, ImageOutputFormat::Bmp);

                let mut return_vec: Vec<u8> = vec![];

                img_buffer.read_to_end(&mut return_vec);

                return return_vec;


}