use windows::{
	core::Interface,
	Win32::Graphics::Direct3D9::{
		IDirect3DDevice9, IDirect3DTexture9, D3DFMT_A8R8G8B8, D3DLOCKED_RECT, D3DPOOL_MANAGED,
	},
};

pub fn create_64x64_d3d9tex(img_data: &mut [u8]) -> *mut IDirect3DTexture9 {
	d3d9_create_tex_from_mem(img_data, 64, 64)
}

// This is the very funni order in which D3DFMT_A8R8G8B8 requires the texture memory stored as.
// BEST PART? THAT LIL HINT IS HIDDEN QUITE DEEP IN THE DOCS..
// AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFFS
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct PixelBGRA {
	b: u8, // blue
	g: u8, // green
	r: u8, // red
	a: u8, // alpha
}

impl PixelBGRA {
	// Format is very dumb: packs of [u8; 4], in this order: [B G R A]...
	// Credit to unknowntrojan & his egui-d3d9 lib: https://github.com/unknowntrojan/egui-d3d9/blob/a0f5ace6b6fc916ba0e9a6077bc17ac359f01663/egui-d3d9/src/texman.rs#L14
	fn bmp_to_bgra_vec(encoded_bmp_image_buffer: &mut [u8]) -> Vec<PixelBGRA> {
		use image::GenericImageView;
		image::load_from_memory_with_format(encoded_bmp_image_buffer, image::ImageFormat::Bmp)
			.unwrap()
			.pixels()
			.map(|(_x, _y, p)| p.0)
			.map(|[r, g, b, a]| PixelBGRA { r, g, b, a })
			.collect()
	}
}

// https://github.com/unknowntrojan/egui-d3d9/blob/a0f5ace6b6fc916ba0e9a6077bc17ac359f01663/egui-d3d9/src/texman.rs#L266
fn d3d9_create_tex_from_mem(
	encoded_bmp_image_buffer: &mut [u8],
	width: u32,
	height: u32,
) -> *mut IDirect3DTexture9 {
	let mut tex_ptr: Option<IDirect3DTexture9> = None;

	let dev: &IDirect3DDevice9 =
		unsafe { &IDirect3DDevice9::from_raw(crate::MyPlugin::get_api().get_d3d9dev()) };

	unsafe {
		// https://learn.microsoft.com/en-us/windows/win32/api/d3d9/nf-d3d9-idirect3ddevice9-createtexture
		// https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dformat
		// https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dpool
		let r = dev.CreateTexture(
			width,
			height,
			1,
			0u32,
			D3DFMT_A8R8G8B8,
			D3DPOOL_MANAGED, // D3DPOOL_MANAGED allows this texture to survive dev.Reset(..)
			&mut tex_ptr,
			std::ptr::null_mut(),
		);
		log::trace!("dev.CreateTexture(tex_ptr: {tex_ptr:?}) -> {r:?}");
		r.expect("dev.CreateTexture failed");
	};
	let tex_ptr = tex_ptr.expect("dev.CreateTexture returned null tex ptr");

	// Then get to writing the inner texture pixel data with tex.LockRect()
	unsafe {
		let src = PixelBGRA::bmp_to_bgra_vec(encoded_bmp_image_buffer);
		let mut rect: D3DLOCKED_RECT = D3DLOCKED_RECT::default();
		tex_ptr
			.LockRect(0, &mut rect, std::ptr::null_mut(), 0)
			.expect("tex_ptr.LockRect(..) failed");
		assert!(width * height == src.len() as u32);
		let dst: &mut [PixelBGRA] =
			std::slice::from_raw_parts_mut(rect.pBits as *mut PixelBGRA, src.len());
		dst.copy_from_slice(&src);
		tex_ptr
			.UnlockRect(0)
			.expect("tex_ptr.UnlockRect(..) failed"); // UPLOAD IT
	}
	log::trace!("Created IDirect3DTexture9: {tex_ptr:?}");
	tex_ptr.into_raw() as *mut IDirect3DTexture9
	// .into_raw() prevents the texture getting cleared by mem::drop().
	// Only the d3d9 device knows about it now
}

/// Strong independent plugin, don't need no "d3dx9_42.dll!D3DXCreateTextureFromFileInMemoryEx(.)"
#[allow(unused)]
#[deprecated]
fn d3d9_create_tex_from_mem_ex_v1(
	tex_buffer: &mut [u8],
	width: u32,
	height: u32,
) -> *mut IDirect3DTexture9 {
	use std::{
		ffi::{c_void, CString},
		iter,
		sync::LazyLock,
	};
	use windows::{
		core::HRESULT,
		Win32::Graphics::Direct3D9::{
			IDirect3DTexture9, D3DFMT_A8R8G8B8, D3DFORMAT, D3DPOOL, D3DPOOL_MANAGED,
		},
	};

	fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
		use windows::{
			core::{PCSTR, PCWSTR},
			Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
		};
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

	log::warn!("D3DXCreateTextureFromFileInMemoryEx(");
	d3d9_func(
		// ptr to IDirect3DDevice9
		crate::MyPlugin::get_api().get_d3d9dev(),
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
		// (default?) 0  usage. idk what 0 means and I'm too scared to look it up
		0u32,
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
	log::warn!(")");
	tex_ptr
}
