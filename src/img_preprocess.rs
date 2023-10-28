use std::io::Read;
use image::{imageops::FilterType::Lanczos3, ImageOutputFormat};
use log::{info, error};
use std::io::Cursor;
use core::fmt::Error;

use anyhow::Result; 
use anyhow::anyhow;

pub fn get_image_from_url(url:String) -> Result<Vec<u8>> {

                let resp = match ureq::get(&url)
                .call() {
                    Ok(resp) => resp,
                    Err(e) => {
                        error!("Failed to make HTTP request: {e}");
                        return Err(anyhow!("Failed to make HTTP request"));
                    },
                };

                let len = match resp
                    .header("Content-Length")
                    .and_then(|s| s.parse::<usize>().ok())
                    {
                        Some(content_size) => content_size,
                        None => {
                            error!("Response from the server is missing Content-Lenght header.");
                            return Err(anyhow!("Response from the server is missing Content-Lenght header."));
                        },
                    };

                let mut bytes: Vec<u8> = Vec::with_capacity(len);

                resp.into_reader()
                    .take(10_000_000)
                    .read_to_end(&mut bytes)
                    .unwrap();

                let mut img = match image::load_from_memory(&bytes) {
                    Ok(img) => img,
                    Err(e) => {
                        error!("Failed to parse downloaded image: {e}");
                        return Err(anyhow!("Failed to parse downloaded image"));
                    },
                };

                img = img.resize(64, 64, Lanczos3);

                let mut return_vec: Vec<u8> = vec![];
                img.write_to(&mut Cursor::new(&mut return_vec), ImageOutputFormat::Bmp).unwrap();

                //img.save_with_format("./debug.bmp", ImageFormat::Bmp);                
                //std::fs::write("./debug2.bmp", &return_vec);

                return Ok(return_vec);


}

