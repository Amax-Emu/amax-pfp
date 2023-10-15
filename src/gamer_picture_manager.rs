use std::{mem, ptr, error::Error};

use log::info;
use retour::static_detour;
use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;
use crate::d3d9_utils::get_d3d9_device;
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

                let filename = std::path::PathBuf::from("./test4.dds");

                let d3d9_device = unsafe { get_d3d9_device() };

                let result = unsafe {crate::d3d9_utils::d3d9_load_texture_from_file(*d3d9_device, texture, img_file_path) };

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