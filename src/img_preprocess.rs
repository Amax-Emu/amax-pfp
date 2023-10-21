use std::io::Read;
use image::{imageops::FilterType::Lanczos3, ImageOutputFormat};
use log::info;
use std::io::Cursor;

//static URL_BASE: String = String::From("https://amax-emu.com/api/");

pub fn get_image_from_url(url:&str) -> Vec<u8> {

                let resp = ureq::get(url)
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

                let mut return_vec: Vec<u8> = vec![];
                //let img2 = img.to_rgb8();
                //let return_vec = img2.to_vec();
                img.write_to(&mut Cursor::new(&mut return_vec), ImageOutputFormat::Bmp).unwrap();

                //img.save_with_format("./debug.bmp", ImageFormat::Bmp);
                
                //std::fs::write("./debug2.bmp", &return_vec);

                return return_vec;


}