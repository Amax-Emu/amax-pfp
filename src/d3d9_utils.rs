use std::{
    ffi::{c_void, CString},
    iter, ptr,
};


use log::{debug, info, warn};
use simplelog::*;
use winapi::shared::d3d9types::D3DCOLOR;
use windows::Win32::Graphics::Direct3D9::IDirect3DDevice9;
use windows::Win32::Graphics::Direct3D9::*;
use windows::{
    core::{HRESULT, PCSTR, PCWSTR},
    Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader::{GetModuleHandleA, GetModuleHandleW, GetProcAddress},
    },
};

type D3DXCreateTextureFromFileA = extern "stdcall" fn(
    device: &IDirect3DDevice9,
    filename: *const u8,
    text: *mut IDirect3DTexture9,
) -> HRESULT;

type D3DXCreateTextureFromFileExA = extern "stdcall" fn(
    device: &IDirect3DDevice9,
    filename: *const u8,
    Width: u32,
    Height: u32,
    MipLevels: u32,
    Usage: u32,
    Format: D3DFORMAT,
    Pool: D3DPOOL,
    Filter: u32,
    MipFilter: u32,
    ColorKey: D3DCOLOR,
    pSrcInfo: *mut c_void,
    pPalette: *mut c_void,
    ppTexture: *mut IDirect3DTexture9,
) -> HRESULT;

type D3DXCreateTextureFromFileInMemory = extern "stdcall" fn(
    pDevice: &IDirect3DDevice9,
    pSrcData: *mut Vec<u8>,
    SrcDataSize: usize,
    ppTexture: *mut IDirect3DTexture9,
) -> HRESULT;

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
    ColorKey: D3DCOLOR,
    pSrcInfo: *mut c_void,
    pPalette: *mut c_void,
    ppTexture: *mut IDirect3DTexture9,
) -> HRESULT;

pub unsafe fn get_d3d9_device() -> *mut IDirect3DDevice9 {
    let start = crate::EXE_BASE_ADDR + 0x00D44EE4;

    let ptr = start as *const i32;
    debug!("Addr of start: {:?}", start);
    debug!("Addr of ptr1: {:p},value: {}", ptr, *ptr);

    if *ptr == 0 {
        //std::thread::sleep(std::time::Duration::from_secs(1));
        warn!("Failed to aquire d3d9 device handle");
        return ptr::null_mut();
    }

    let step2 = *ptr;

    let step3 = step2 + 0x14;

    let step4 = step3 as *const i32;
    debug!("Addr of step4: {:p},value: {}", step4, *step4);
    let d3d9_ptr_real = *step4 as *mut IDirect3DDevice9;
    info!("Addr of d3d device: {:p}", d3d9_ptr_real);

    return d3d9_ptr_real;
}

pub fn d3d9_load_texture_from_file(
    texture_ptr: *mut IDirect3DTexture9,
    file_path: &str,
) -> Result<(), ()> {
    // let filename = String::from("./test4.dds");
    // let filename_bytes = filename.as_bytes().to_owned();

    let device = unsafe { get_d3d9_device() };

    let func_addr = get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileA")
        .expect("could not find 'D3DXCreateTextureFromFileA' address");

    let d3d9_func: D3DXCreateTextureFromFileA = unsafe { std::mem::transmute(func_addr) };

    let filename = String::from(file_path);
    let filename_bytes = filename.as_bytes().to_owned();
    unsafe {
        let result = d3d9_func(
            &*device,
            ptr::addr_of!(filename_bytes[0]),
            texture_ptr,
        );

        debug!("Result of D3DXCreateTextureFromFileA: {:?}", &result);

        if result.is_ok() {
            Ok(())
        } else {
            Err(())
        }
    }
}

pub fn d3d9_load_texture_from_file_ex(
    texture_ptr: *mut IDirect3DTexture9,
    file_path: &str, //I'll change it to path, promise
    width: u32,
    height: u32,
) -> Result<(), ()> {
    let func_addr = get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileExA")
        .expect("could not find 'D3DXCreateTextureFromFileExA' address");
    info!("D3DXCreateTextureFromFileExA addr: {}",func_addr);
    let d3d9_func: D3DXCreateTextureFromFileExA = unsafe { std::mem::transmute(func_addr) };

    let device = unsafe { get_d3d9_device() };

    let filename = String::from(file_path);
    let filename_bytes = filename.as_bytes().to_owned();

    unsafe {
        let result = d3d9_func(
            &*device,
            ptr::addr_of!(filename_bytes[0]),
            width,
            height,
            1,
            0,
            D3DFORMAT(20), //dxt1 is rather noisy, so rgb8
            D3DPOOL(1),
            1,
            1,
            0xFF000000,
            ptr::null_mut(),
            ptr::null_mut(),
            texture_ptr,
        );

        debug!("Result of D3DXCreateTextureFromFileExA: {:?}", &result);

        if result.is_ok() {
            Ok(())
        } else {
            Err(())
        }
    }
}

pub fn d3d9_load_texture_from_memory_ex(texture_ptr: *mut IDirect3DTexture9, mut tex_buffer: Vec<u8>,width:u32,height:u32) -> Result<(),()> {

    let func_addr = get_module_symbol_address(
        "d3dx9_42.dll",
        "D3DXCreateTextureFromFileInMemoryEx",
    )
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

    debug!("Result of D3DXCreateTextureFromFileInMemoryEx: {:?}", &result);

    if result.is_ok() {
        Ok(())
    } else {
        Err(())
    }
}

}

// unsafe fn legacy_create_texture() {
//     let mut new_gpu: *mut IDirect3DDevice9 = ptr::null_mut();
//     // let EXE_BASE_ADDR = 0x00400000;
//     // let mut addr = EXE_BASE_ADDR + 0x00D44EE4;
//     let EXE_BASE_ADDR = 0x00400000;

//     let start = EXE_BASE_ADDR + 0x00D44EE4;

//     let ptr = start as *const i32;
//     info!("Addr of start: {:?}", start);
//     info!("Addr of ptr1: {:p},value: {}", ptr, *ptr);

//     if *ptr == 0 {
//         std::thread::sleep(std::time::Duration::from_secs(1));
//     }

//     let step2 = *ptr;

//     let step3 = step2 + 0x14;

//     let step4 = step3 as *const i32;
//     info!("Addr of step4: {:p},value: {}", step4, *step4);
//     let d3d9_ptr_real = *step4 as *mut IDirect3DDevice9;
//     info!("Addr of d3d device_real: {:p}", d3d9_ptr_real);

//     let d3d9_ptr = step3 as *mut IDirect3DDevice9;
//     info!("Addr of d3d device: {:p}", d3d9_ptr);

//     let mut text: Option<IDirect3DTexture9> = None;
//     info!("Addr of texture: {:p}", ptr::addr_of_mut!(text));
//     let result = IDirect3DDevice9::CreateTexture(
//         &*d3d9_ptr,
//         64,
//         64,
//         1,
//         0,
//         D3DFORMAT(827611204),
//         D3DPOOL(1),
//         ptr::addr_of_mut!(text),
//         ptr::null_mut(),
//     );
//     info!("Result: {:?}", result);

//     let address = get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileA")
//         .expect("could not find 'D3DXCreateTextureFromFileA' address");
//     info!("Addr of D3DXCreateTextureFromFileA: {}", address);

//     let filename = String::from("./test.bmp");
//     let filename_bytes = filename.as_bytes().to_owned();
//     type D3DXCreateTextureFromFileA = extern "stdcall" fn(
//         device: &IDirect3DDevice9,
//         filename: *const u8,
//         text: *mut IDirect3DTexture9,
//     ) -> HRESULT;

//     let mut text2: IDirect3DTexture9 = text.unwrap();

//     let my_func: D3DXCreateTextureFromFileA = std::mem::transmute(address);

//     let result = my_func(
//         &*d3d9_ptr_real,
//         ptr::addr_of!(filename_bytes[0]),
//         ptr::addr_of_mut!(text2),
//     );
//     info!("Addr of texture: {:p}", ptr::addr_of_mut!(text2));
//     let hook1 = ptr::addr_of_mut!(text2) as *mut i32;
//     info!("REAL Addr of texture: {:?}", *hook1);
//     info!("Result: {:?}", result);
// }

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
