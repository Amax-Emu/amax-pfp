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

pub unsafe fn get_d3d9_device() -> *mut IDirect3DDevice9 {
    let start = crate::EXE_BASE_ADDR + 0x00D44EE4;

    let ptr = start as *const i32;
    log::debug!("Addr of start: {:?}", start);
    log::debug!("Addr of ptr1: {:p},value: {}", ptr, *ptr);

    if *ptr == 0 {
        //std::thread::sleep(std::time::Duration::from_secs(1));
        log::warn!("Failed to aquire d3d9 device handle");
        return ptr::null_mut();
    }

    let step2 = *ptr;

    let step3 = step2 + 0x14;

    let step4 = step3 as *const i32;
    log::debug!("Addr of step4: {:p},value: {}", step4, *step4);
    let d3d9_ptr_real = *step4 as *mut IDirect3DDevice9;
    log::info!("Addr of d3d device: {:p}", d3d9_ptr_real);

    return d3d9_ptr_real;
}

pub fn d3d9_load_texture_from_memory_ex(
    texture_ptr: *mut IDirect3DTexture9,
    mut tex_buffer: Vec<u8>,
    width: u32,
    height: u32,
) -> Result<(), ()> {
    let func_addr =
        get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileInMemoryEx")
            .expect("could not find 'D3DXCreateTextureFromFileInMemoryEx' address");

    let d3d9_func: D3DXCreateTextureFromFileInMemoryEx = unsafe { std::mem::transmute(func_addr) };
    let device = unsafe { get_d3d9_device() };
    unsafe {
        let result = d3d9_func(
            &*device,
            ptr::addr_of_mut!(tex_buffer[0]), //todo: fix this
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

        log::debug!(
            "Result of D3DXCreateTextureFromFileInMemoryEx: {:?}",
            &result
        );

        if result.is_ok() {
            Ok(())
        } else {
            Err(())
        }
    }
}

pub fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
    let module = module
        .encode_utf16()
        .chain(iter::once(0))
        .collect::<Vec<u16>>();
    let symbol = CString::new(symbol).unwrap();
    unsafe {
        let handle = GetModuleHandleW(PCWSTR(module.as_ptr() as _)).unwrap();
        match GetProcAddress(handle, PCSTR(symbol.as_ptr() as _)) {
            Some(func) => Some(func as usize),
            None => None,
        }
    }
}
