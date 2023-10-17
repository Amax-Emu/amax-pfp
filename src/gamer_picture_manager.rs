use std::{error::Error, mem, ptr};

use crate::{d3d9_utils::get_d3d9_device, img_preprocess::get_image_from_url};
use log::info;
use retour::static_detour;
use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;
#[derive(Debug)]
#[repr(C)]
struct C_GamerPicture {
    //total size on pc: 80
    unk0: u32, //0x4C 0xA8, 0xEA, 0x00,
    small_unk0: u8,
    Ref_aka_pad_id: u8,                 //0x00
    UserInformation: [u8; 18], // 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00
    active: bool,              // 0x00
    free: bool,                // 0x01
    GamerPicName: [u8; 30],    //GAMERPIC_X or REMOTE_GAMERPIC_X
    size_as_big_end_temp: u32, // 0x00, 0x00, 0x00, 0x00
    unk_zeroes: u32,           // 0x00, 0x40 0x00, 0x00,
    unk_4_as_u16: u16,         //0x04, 0x00,
    new_texture_ptr: IDirect3DTexture9, //0xE0, 0x71 0x90, 0x14
    default_texture_ptr: u32,  //   0xB0, 0xCB 0x40, 0x0F
    unk4: u32,                 // 0x00, 0x00
}



use std::{
    ffi::{c_void, CString},
    iter,
};



use simplelog::*;
use winapi::shared::{d3d9types::D3DCOLOR, ntdef::LPCSTR};
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

static_detour! {
  static GetPrimaryProfilePictureHook: unsafe extern "system" fn() -> bool;
}

static_detour! {
    static GamePictureManager_CreateHook: unsafe extern "system" fn(i32,i32,*const [u8;32],bool) -> bool;
}

pub unsafe fn create_get_primary_profile_picture_hook() {
    let address = 0x00d5e170;
    let target = mem::transmute(address);
    GetPrimaryProfilePictureHook
        .initialize(target, primary_picture_load)
        .unwrap()
        .enable()
        .unwrap();
}

pub unsafe fn create_gamer_picture_manager_hook() {
    let address = 0x0079dc50; //gamerpicmanager_create
    let target = mem::transmute(address);
    GamePictureManager_CreateHook
        .initialize(target, manager_create)
        .unwrap()
        .enable()
        .unwrap();
}

fn manager_create(
    max_local: i32,
    max_remote: i32,
    default_texture: *const [u8; 32],
    small: bool,
) -> bool {
    info!("max_local: {max_local}, max_remote:{max_remote},default_texture: {default_texture:?},small;:{small} ");
    return true;
}

unsafe fn get_local_gamerpic() -> *mut [C_GamerPicture; 4] {
    //todo: rework pointer, this one is not very stable

    let exe_base_addr = 0x00400000;
    let local_start = exe_base_addr + 0x00D61518;
    info!("Addr of start: {:?}", local_start);

    let ptr = local_start as *const i32;

    let ptr = *ptr as *mut [C_GamerPicture; 4];
    info!("Addr of start: {:?}", local_start);
    info!("Addr of local pictures ptr: {:p}", &ptr);
    return ptr;
}

fn pretty_name(name_buf: &[u8]) -> String {
    let name = String::from_utf8(name_buf.to_vec()).unwrap();
    return name.trim_matches(char::from(0)).to_string();
}

fn primary_picture_load() -> bool {
    info!("GetPrimaryProfilePicture detour!");
    unsafe {
        let local_gamerpics = get_local_gamerpic();

        for picture in &mut *local_gamerpics {
            info!(
                "Addr of picture: {:p} Data: {:?}",
                ptr::addr_of!(picture),
                picture
            );
            let mut name = pretty_name(&picture.GamerPicName);
            info!("{}", &name);

            if name == "GAMERPIC_0" {
                info!("Loading primary picture");
                //let filename = std::path::PathBuf::from("./test.bmp");
                

                // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!! WORKING CODE DO NOT TOUCH

                // let img_data = get_image_from_url();

                // let func_addr =
                //     get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileExA")
                //         .expect("could not find 'D3DXCreateTextureFromFileExA' address");

                // info!("D3DXCreateTextureFromFileExA addr: {}", func_addr);

                // let d3d9_func: D3DXCreateTextureFromFileExA =
                //     unsafe { std::mem::transmute(func_addr) };

                // //let device = unsafe { get_d3d9_device() };

                // let filename = String::from("./test4.dds");
                // let filename_bytes = filename.as_bytes().to_owned();


                // let start = 0x00400000 + 0x00D44EE4;

                // let ptr = start as *const i32;
                // info!("Addr of start: {:?}", start);
                // info!("Addr of ptr1: {:p},value: {}", ptr, *ptr);

                // let step2 = *ptr;

                // let step3 = step2 + 0x14;

                // let step4 = step3 as *const i32;
                // info!("Addr of step4: {:p},value: {}", step4, *step4);
                // let d3d9_ptr_real = *step4 as *mut IDirect3DDevice9;
                // info!("Addr of d3d device_real: {:p}", d3d9_ptr_real);


                // let result = d3d9_func(
                //     &*d3d9_ptr_real,
                //     ptr::addr_of!(filename_bytes[0]),
                //     64,
                //     64,
                //     1,
                //     0,
                //     D3DFORMAT(827611204),
                //     D3DPOOL(1),
                //     1,
                //     1,
                //     0xFF000000,
                //     ptr::null_mut(),
                //     ptr::null_mut(),
                //     ptr::addr_of_mut!(picture.new_texture_ptr),
                // );

                // info!("Result of D3DXCreateTextureFromFileExA: {:?}", &result);


                // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ WORKING CODE DO NOT TOUCH   

                //let result = crate::d3d9_utils::d3d9_load_texture_from_file(picture.new_texture_ptr.clone(), &filename) ;
                

                //NOTE TO SELF: CLONE DOESN'T WORK ON IDirect3DTexture9. PASS A PTR

                let result = crate::d3d9_utils::d3d9_load_texture_from_file_ex(ptr::addr_of_mut!(picture.new_texture_ptr), "./test4.dds",64,64) ;

                info!("Result: {:?}", result);

                if result.is_err() {
                    panic!();
                }

                picture.active = true;
            }
        }
    }

    return false;
}

// fn fill_gamerpic_texture_from_file(texture: IDirect3DTexture9,img_file_path: std::path::Path) -> Result<(),()> {

//     return result;

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
