use std::{
	ffi::{c_void, CString},
	iter, ptr,
};
use windows::Win32::Graphics::Direct3D9::{
	IDirect3DDevice9, IDirect3DTexture9, D3DFORMAT, D3DPOOL,
};

use windows::{
	core::{HRESULT, PCSTR, PCWSTR},
	Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
};

type D3DXCreateTextureFromFileInMemoryEx = extern "stdcall" fn(
	device: &IDirect3DDevice9,
	pSrcData: *mut u8,
	SrcDataSize: usize,
	Width: u32,
	Height: u32,
	MipLevels: u32,
	Usage: u32,
	Format: D3DFORMAT,
	Pool: D3DPOOL,
	Filter: u32,
	MipFilter: u32,
	ColorKey: u32,
	pSrcInfo: *mut c_void,
	pPalette: *mut c_void,
	ppTexture: *mut IDirect3DTexture9,
) -> HRESULT;

unsafe fn get_d3d9_device(ptr_base: *mut c_void) -> *mut IDirect3DDevice9 {
	const OFFSET_D3D9DEV_PTR_PATH_START: isize = 0x00D44EE4;
	const OFFSET_D3D9DEV_PTR_PATH_L0: isize = 0x14;
	let start: *mut *mut *mut IDirect3DDevice9 =
		ptr_base.wrapping_offset(OFFSET_D3D9DEV_PTR_PATH_START) as _;
	start
		.read()
		.wrapping_byte_offset(OFFSET_D3D9DEV_PTR_PATH_L0)
		.read()
}

pub fn d3d9_create_tex_from_mem_ex(
	texture_ptr: *mut IDirect3DTexture9,
	tex_buffer: &mut [u8],
	width: u32,
	height: u32,
) -> Result<(), ()> {
	let d3d9_func: D3DXCreateTextureFromFileInMemoryEx = {
		let func_addr =
			get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileInMemoryEx")
				.expect("could not find 'D3DXCreateTextureFromFileInMemoryEx' address");
		unsafe { std::mem::transmute(func_addr) }
	};
	let device: &IDirect3DDevice9 = unsafe {
		get_d3d9_device(crate::CoolBlurPlugin::get_exe_base_ptr())
			.as_ref()
			.unwrap()
	};
	let result = d3d9_func(
		device,
		tex_buffer.as_mut_ptr(),
		tex_buffer.len(),
		width,
		height,
		1,
		0,
		D3DFORMAT(20),
		D3DPOOL(1),
		1,
		1,
		0xFF000000,
		ptr::null_mut(),
		ptr::null_mut(),
		texture_ptr,
	);

	log::debug!("Result of D3DXCreateTextureFromFileInMemoryEx: {result:?}");

	result.ok().or(Err(()))
}

pub fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
	let module = module
		.encode_utf16()
		.chain(iter::once(0))
		.collect::<Vec<u16>>();
	let symbol = CString::new(symbol).unwrap();
	unsafe {
		let handle = GetModuleHandleW(PCWSTR(module.as_ptr() as *const _)).unwrap();
		GetProcAddress(handle, PCSTR(symbol.as_ptr() as _)).map(|addr| addr as usize)
	}
}
