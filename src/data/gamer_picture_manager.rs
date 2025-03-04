use std::ffi::c_void;
use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GamerPictureManager {
	thread: [u8; 20], // C8 AA EA 00 00 00 00 00 00 00 00 00 0D F0 AD BA 0D F0 AD BA || This is a thread pointer at PS3. I don't know what is it on PC.
	pub local_pictures_ptr: *const [*mut C_GamerPicture; 4],
	pub local_pictures_size: usize,
	pub local_pictures_len: usize, //this one is used in GetTotalPicturesFunctions
	pub remote_pictures_ptr: *const [*mut C_GamerPicture; 19], //hardcoding to 19
	pub remote_pictures_size: usize, // Has value of 26. Bug? Cut feature? We will never know.
	pub remote_pictures_len: usize, //this one is used in GetTotalPicturesFunctions
}

impl GamerPictureManager {
	//FIXME: Rust aliasing rules make this kind of &mut T reference unsound (*SOMETIMES!*)
	// The compiler can assume that there is nothing else pointing to T, and do stuff we don't want.
	// I don't know how likely that is tho.
	// I think in most cases here we should be fine.
	// It would be better to just return the raw pointer  -> `Option<*mut GamerPictureManager>`
	// I hate working the *mut T though, all the unsafe and all the dereferencing with (*T) is annoying.
	// Also this whole DLL is unsafe and Blur is unsafe AND WHO IS EVEN GONNA READ THIS LALALAlalalalaa
	// ITS MY PULL REQUEST I CAN DO WHAT I WANT
	pub fn summon<'a>(ptr_base: *mut c_void) -> Option<&'a mut Self> {
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
}

#[derive(Debug)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct C_GamerPicture {
	//total size on pc: 80
	unk_ptr0: u32, //0x4C 0xA8, 0xEA, 0x00,
	pub ref1: u16, // when this matches a NetRacer.mp_lobby_ref_id, the good things happen
	pub user_dw_id: u64,
	user_information: [u8; 8], // 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00
	pub active: bool,          // 0x00
	pub free: bool,            // 0x01
	pub gamer_pic_name: [u8; 30], //GAMERPIC_X or REMOTE_GAMERPIC_X
	size_as_big_end_temp: u32, // 0x00, 0x00, 0x00, 0x00
	unk_zeroes: u32,           // 0x00, 0x40 0x00, 0x00,
	unk_4_as_u16: u16,         //0x04, 0x00,
	pub texture_ptr: *mut IDirect3DTexture9, //0xE0, 0x71 0x90, 0x14
	pub default_texture_ptr: u32, //   0xB0, 0xCB 0x40, 0x0F
	unk4: u32,                 // 0x00, 0x00
}

impl C_GamerPicture {
	pub fn get_name(&self) -> String {
		let name = String::from_utf8(self.gamer_pic_name.to_vec()).unwrap();
		name.trim_matches(char::from(0)).to_string()
	}
}
