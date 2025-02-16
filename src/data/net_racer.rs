use std::ffi::c_void;

use widestring::WideCString;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct NetPlayer {
	// this structure contain a lot of usefull data, but we're not interested
	unk0: [u8; 0x4],
	ptr_to_next: *mut NetPlayer, // 0x4
	unk1: [u8; 0x40],            // 0x4
	user_dw_id: u64,             //+0x48
	zeroes: [u8; 8],
	username_in_utf_16: [u16; 16], // +0x58
	unk2: [u8; 164],
	mp_lobby_ref_id: u8, //position of user in mplobby and the exact value remote_picture ref should be set to
	unk3: [u8; 107],
}

impl NetPlayer {
	// There is a Linked List of NetRacers
	// This is how you can obtain the first NetPlayer in the list
	// NOTE: This might be the first time in my lifeI find an actual Linked List that is somewhat useful in the wild!
	pub fn get_first_lobby_net_racer<'a>(ptr_base: *mut c_void) -> Option<&'a mut Self> {
		let p: *mut *mut *mut Self = ptr_base.wrapping_byte_offset(0xDB4530) as _;
		// * -> [[[Blur.exe + 0xDB4530] + 0x18] + 0]
		unsafe {
			let p: *mut Self = p.read().wrapping_byte_offset(0x18).read();
			if p.is_null() {
				return None;
			}
			Some(&mut (*p))
		}
	}

	pub fn get_next(&self) -> Option<&mut NetPlayer> {
		if self.ptr_to_next.is_null() {
			None
		} else {
			Some(unsafe { &mut *self.ptr_to_next })
		}
	}
	pub fn get_dw_id(&self) -> u64 {
		self.user_dw_id
	}
	pub fn get_username(&self) -> String {
		WideCString::from_vec_truncate(self.username_in_utf_16)
			.to_string()
			.unwrap()
	}

	pub fn get_lobby_ref(&self) -> u8 {
		self.mp_lobby_ref_id
	}
}
