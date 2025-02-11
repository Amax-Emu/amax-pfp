use std::{
	ffi::{c_void, CString},
	iter,
	sync::LazyLock,
};
use windows::Win32::Graphics::Direct3D9::{
	IDirect3DTexture9, D3DFMT_X8R8G8B8, D3DFORMAT, D3DPOOL, D3DPOOL_MANAGED,
};

use windows::{
	core::{HRESULT, PCSTR, PCWSTR},
	Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
};

type D3DXCreateTextureFromFileInMemoryEx = extern "stdcall" fn(
	device: *mut c_void,
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
	ppTexture: *mut *mut IDirect3DTexture9,
) -> HRESULT;

pub fn d3d9_create_tex_from_mem_ex(
	tex_buffer: &mut [u8],
	width: u32,
	height: u32,
) -> *mut IDirect3DTexture9 {
	static ONCE_FN_D3DX_CREATE_TEXTURE_FROM_FILE_IN_MEMORY_EX: LazyLock<
		D3DXCreateTextureFromFileInMemoryEx,
	> = LazyLock::new(|| {
		let func_addr =
			get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileInMemoryEx")
				.expect("could not get_module_symbol_address() for 'D3DXCreateTextureFromFileInMemoryEx(..)' function in 'd3dx9_42.dll'");
		unsafe { std::mem::transmute::<usize, D3DXCreateTextureFromFileInMemoryEx>(func_addr) }
	});
	let d3d9_func = *ONCE_FN_D3DX_CREATE_TEXTURE_FROM_FILE_IN_MEMORY_EX;

	let mut tex_ptr: *mut IDirect3DTexture9 = std::ptr::null_mut();

	d3d9_func(
		// ptr to IDirect3DDevice9
		crate::CoolBlurPlugin::get_api().get_d3d9dev(),
		// ptr to bytes img data
		tex_buffer.as_mut_ptr(),
		// size of file in mem
		tex_buffer.len(),
		// image width
		width,
		// image height
		height,
		// mipLevels
		1,
		// (default?) usage. idk what 0 means and I'm too scared to look it up
		0,
		// D3DFMT_R8G8B8 | https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dformat
		D3DFMT_X8R8G8B8,
		// D3DPOOL_MANAGED | https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dpool
		D3DPOOL_MANAGED,
		// .filter = D3DX_FILTER_NONE  | https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dx-filter
		1,
		// .MipFilter = D3DX_FILTER_NONE
		1,
		// .ColorKey = D3DCOLOR, 32bit ARGB, opaque black
		0xFF000000,
		// pSrcInfo D3DXIMAGE_INFO structure to be filled with a description of the data in the source image file, or NULL
		std::ptr::null_mut(),
		// pPalette Pointer to a PALETTEENTRY structure, representing a 256-color palette to fill in, or NULL
		std::ptr::null_mut(),
		// ppTexture | Address of a pointer to an IDirect3DTexture9 interface, representing the created texture object.
		&mut tex_ptr,
	)
	.unwrap();

	tex_ptr
}

fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
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
