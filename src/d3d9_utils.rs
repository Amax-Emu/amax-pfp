use image::{load_from_memory_with_format, GenericImageView};
use std::{
	ffi::{c_void, CString},
	iter,
	sync::LazyLock,
};
use windows::Win32::Graphics::Direct3D9::{
	IDirect3DDevice9, IDirect3DTexture9, D3DFMT_A8R8G8B8, D3DFORMAT, D3DLOCKED_RECT,
	D3DLOCK_DISCARD, D3DLOCK_READONLY, D3DPOOL, D3DPOOL_DEFAULT, D3DPOOL_MANAGED,
	D3DPOOL_SYSTEMMEM, D3DUSAGE_DYNAMIC,
};

use windows::{
	core::Interface,
	core::{HRESULT, PCSTR, PCWSTR},
	Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
};

pub fn create_64x64_d3d9tex(img_data: &mut [u8]) -> *mut IDirect3DTexture9 {
	d3d9_create_tex_from_mem_ex(img_data, 64, 64)
	// d3d9_create_tex_from_mem_ex_v2(img_data, 64, 64) //FIXME
}

#[allow(unused)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct RetardedBGRA {
	b: u8,
	g: u8,
	r: u8,
	a: u8,
}

#[allow(unused)]
fn to_brga(encoded_bmp_image_buffer: &mut [u8]) -> Vec<RetardedBGRA> {
	let imga =
		load_from_memory_with_format(&encoded_bmp_image_buffer, image::ImageFormat::Bmp).unwrap();
	let r: Vec<RetardedBGRA> = imga
		.pixels()
		.map(|(_x, _y, p)| p.0)
		.map(|[r, g, b, a]| RetardedBGRA { r, g, b, a })
		.collect();
	log::info!("imgazie {:?} igmalen {}", imga.dimensions(), r.len());
	r
}

// https://github.com/unknowntrojan/egui-d3d9/blob/a0f5ace6b6fc916ba0e9a6077bc17ac359f01663/egui-d3d9/src/texman.rs#L266
// FIXME: I really wanna try this
#[allow(unused)]
fn d3d9_create_tex_from_mem_ex_v2(
	encoded_bmp_image_buffer: &mut [u8],
	width: u32,
	height: u32,
) -> *mut IDirect3DTexture9 {
	let mut tex_ptr: Option<IDirect3DTexture9> = None;
	let mut tex_ptr2: Option<IDirect3DTexture9> = None;

	let dev: &IDirect3DDevice9 =
		unsafe { &IDirect3DDevice9::from_raw(crate::CoolBlurPlugin::get_api().get_d3d9dev()) };

	log::warn!("We CreateTexture...");

	// CreateTexture
	unsafe {
		// https://learn.microsoft.com/en-us/windows/win32/api/d3d9/nf-d3d9-idirect3ddevice9-createtexture
		// https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dformat
		// https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dpool
		let r = dev.CreateTexture(
			width,
			height,
			1,
			D3DUSAGE_DYNAMIC as _,
			D3DFMT_A8R8G8B8,
			D3DPOOL_SYSTEMMEM,
			&mut tex_ptr,
			std::ptr::null_mut(),
		);
		r.unwrap();
	};
	log::info!("we made it past tex_ptr.unwrap()");
	let tex_ptr = tex_ptr.unwrap();
	log::info!("{tex_ptr:?} -> {:?}", tex_ptr.as_raw());

	// TODO: Convert encoded_bmp_image_buffer to Vec<u8>
	// Format is very dumb: packs of [u8; 4], in this dumb ass order: [B G R A]...
	// https://github.com/unknowntrojan/egui-d3d9/blob/a0f5ace6b6fc916ba0e9a6077bc17ac359f01663/egui-d3d9/src/texman.rs#L14
	// placeholder:
	let tex_data_len = width * height * 4;
	let tex_data: Vec<u8> = vec![255, tex_data_len as _];

	// Then get the inner texture pixel data with LockRect
	unsafe {
		let mut rect: D3DLOCKED_RECT = D3DLOCKED_RECT::default();
		let lock_flags = D3DLOCK_DISCARD as u32 | D3DLOCK_READONLY as u32;
		tex_ptr
			.LockRect(0, &mut rect, std::ptr::null_mut(), lock_flags)
			.unwrap();
		log::info!("We made past LockRect");
		/*
		let dst: &mut [u8] = std::slice::from_raw_parts_mut(rect.pBits as *mut u8, tex_data.len());
		dst.copy_from_slice(&tex_data);
		*/
		let src = to_brga(encoded_bmp_image_buffer);
		let dst: &mut [RetardedBGRA] =
			std::slice::from_raw_parts_mut(rect.pBits as *mut RetardedBGRA, src.len());
		dst.copy_from_slice(&src);
		tex_ptr.UnlockRect(0).unwrap();
	}
	// might want to try:
	// https://github.com/unknowntrojan/egui-d3d9/blob/a0f5ace6b6fc916ba0e9a6077bc17ac359f01663/egui-d3d9/src/texman.rs#L331

	log::warn!("creating update tex");
	// let tex_ptr = tex_ptr.as_raw() as *mut IDirect3DTexture9;
	unsafe {
		let r = dev.CreateTexture(
			width,
			height,
			1,
			D3DUSAGE_DYNAMIC as _,
			D3DFMT_A8R8G8B8,
			D3DPOOL_DEFAULT,
			&mut tex_ptr2,
			std::ptr::null_mut(),
		);
		r.unwrap();
	};
	let tex_ptr2 = tex_ptr2.unwrap();
	unsafe {
		let r = dev.UpdateTexture(&tex_ptr, &tex_ptr2);
		r.unwrap();
	};
	log::trace!("We survived d3d9_create_tex_from_mem_ex_v2! {tex_ptr2:?}");
	tex_ptr2.as_raw() as *mut IDirect3DTexture9
}

#[allow(unused)]
// #[deprecated]
fn d3d9_create_tex_from_mem_ex(
	tex_buffer: &mut [u8],
	width: u32,
	height: u32,
) -> *mut IDirect3DTexture9 {
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
		D3DFMT_A8R8G8B8,
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
