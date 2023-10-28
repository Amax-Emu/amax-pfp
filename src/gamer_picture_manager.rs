use crate::img_preprocess::get_image_from_url;
use anyhow::anyhow;
use anyhow::Result;
use core::fmt::Error;
use log::{error, info};
use retour::static_detour;
use std::{collections::HashMap, fs, io, mem, ptr, str::Utf8Error};
use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

//static URL_BASE: String = String::from("https://amax-emu.com/api");

//static PFP_CACHE: HashMap<String, Vec<u8>> = Lazy::new(||HashMap::new());

#[derive(Debug)]
#[repr(C)]
struct C_GamerPicture {
    //total size on pc: 80
    unk_ptr0: u32, //0x4C 0xA8, 0xEA, 0x00,
    small_unk0: u8,
    reference_pad_id: u8,           //0x00
    user_information: [u8; 18], // 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00
    active: bool,               // 0x00
    free: bool,                 // 0x01
    gamer_pic_name: [u8; 30],   //GAMERPIC_X or REMOTE_GAMERPIC_X
    size_as_big_end_temp: u32,  // 0x00, 0x00, 0x00, 0x00
    unk_zeroes: u32,            // 0x00, 0x40 0x00, 0x00,
    unk_4_as_u16: u16,          //0x04, 0x00,
    texture_ptr: IDirect3DTexture9, //0xE0, 0x71 0x90, 0x14
    default_texture_ptr: u32,   //   0xB0, 0xCB 0x40, 0x0F
    unk4: u32,                  // 0x00, 0x00
}

static_detour! {
  static GetPrimaryProfilePictureHook: unsafe extern "system" fn() -> bool;
}

static_detour! {
    static GamePictureManager_CreateHook: unsafe extern "system" fn(i32,i32,*const [u8;32],bool) -> bool;
}

//00b86d20
static_detour! {
    static GamePictureManager_RequestRemotePicture: unsafe extern "system" fn(i32) -> bool;
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

pub unsafe fn create_request_remote_picture_game_hook() {
    let address = 0x00b86d20;
    let target = mem::transmute(address);
    GamePictureManager_RequestRemotePicture
        .initialize(target, request_remote_picture)
        .unwrap()
        .enable()
        .unwrap();
}

fn request_remote_picture(unk1: i32) -> bool {
    info!("unk1: {unk1}");
    return true;
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

    let local_start = crate::EXE_BASE_ADDR + 0x00D61518;
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
    info!("GetPrimaryProfilePicture hook");
    unsafe {
        let local_gamerpics = get_local_gamerpic();

        for picture in &mut *local_gamerpics {
            info!(
                "Addr of picture: {:p} Data: {:?}",
                ptr::addr_of!(picture),
                picture
            );

            let name = pretty_name(&picture.gamer_pic_name);
            info!("Processing {}", &name);

            //TODO: map gamerpics to (fx)hashmap to speedup?
            if name == "GAMERPIC_0" {
                info!("Loading primary picture for user");

                let username = match get_saved_profile_username() {
                    Ok(username) => username.to_string(),
                    Err(e) => {
                        error!("Failed to get username: {e}. Skipping pfp setup.");
                        continue;
                    }
                };

                //let img_data = get_pfp_for_usename(username);

                //something with match goes here

                // WORKING CODE

                //let result = crate::d3d9_utils::d3d9_load_texture_from_file(ptr::addr_of_mut!(picture.texture_ptr), "./test4.dds") ;

                //NOTE TO SELF: CLONE DOESN'T WORK ON IDirect3DTexture9. PASS A PTR

                //let result = crate::d3d9_utils::d3d9_load_texture_from_file_ex(ptr::addr_of_mut!(picture.texture_ptr), "./test4.dds",64,64) ;

                //let img_data = std::fs::read("./test.bmp").unwrap();

                // WORKING CODE

                //let img_data = get_image_from_url("https://cdn.discordapp.com/avatars/925665499692544040/483eb1b92db6a449a0e2bed9a8b48bb3.png");

                let img_data = match get_primary_profile_pic(&username) {
                    Ok(img_data) => img_data,
                    Err(e) => {
                        error!("Failed to get local image {e}");
                        continue;
                    }
                };

                let result = crate::d3d9_utils::d3d9_load_texture_from_memory_ex(
                    ptr::addr_of_mut!(picture.texture_ptr),
                    img_data,
                    64,
                    64,
                );

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

// pub fn create_local_pfp(name: impl AsRef<Path>) -> Result<File, io::Error> {
// 	let dir = known_folders::get_known_folder_path(known_folders::KnownFolder::RoamingAppData)
// 		.ok_or_else(|| io::Error::other("Couldn't get %APPDATA%/Roaming as a KnownFolder"))?
// 		.join("bizarre creations")
// 		.join("blur")
// 		.join("amax")
// 		.join("log");
// 	if !&dir.is_dir() {
// 		fs::create_dir_all(&dir)?;
// 	}
// 	let log_file = dir.join(name);
// 	File::create(log_file)
// }

//Yes, we gonna copy-paste same code for retreiving username from project to project, why are you asking?
pub fn get_saved_profile_username() -> Result<String, Utf8Error> {
    use std::ffi::{c_char, CStr};
    use windows::{core::PCSTR, Win32::System::LibraryLoader::GetModuleHandleA};

    let ptr_base: *mut std::ffi::c_void =
        unsafe { GetModuleHandleA(PCSTR::null()) }.unwrap().0 as _;

    // "Blur.exe"+0xE144E1
    const OFFSET_PROFILE_USERNAME: isize = 0xE144E1;

    let ptr = ptr_base.wrapping_offset(OFFSET_PROFILE_USERNAME) as *const c_char;
    let s = unsafe { CStr::from_ptr(ptr) };
    match s.to_str() {
        Ok(str) => Ok(str.to_string()),
        Err(e) => {
            error!("Could not read username as UTF-8 str from profile.");
            Err(e)
        }
    }
}

fn get_primary_profile_pic(username: &str) -> Result<Vec<u8>> {
    //first we try to get picture though HTTP

    let dir = known_folders::get_known_folder_path(known_folders::KnownFolder::RoamingAppData)
        .ok_or_else(|| io::Error::other("Couldn't get %APPDATA%/Roaming as a KnownFolder"))
        .unwrap()
        .join("bizarre creations")
        .join("blur")
        .join("amax");

    if !&dir.is_dir() {
        match fs::create_dir_all(&dir) {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to create amax folder in AppData: {e}");
                return Err(anyhow!("Failed to create amax folder in AppData"));
            }
        }
    };

    let local_pfp_path = &dir.join("./pfp.bmp");

    match get_pfp_via_http_for_username(username) {
        Ok(img_data) => {
            std::fs::write(local_pfp_path, &img_data);
            return Ok(img_data);
        }
        Err(e) => {
            error!("Failed to get image via http: {e}");
        }
    };

    //secod attempt to fetch it from local cache, located in AppData

    match std::fs::read(local_pfp_path) {
        Ok(img_data) => Ok(img_data),
        Err(e) => {
            error!("Failed to read image data: {e}");
            Err(anyhow!("Failed to read image data."))
        }
    }
}

fn get_pfp_via_http_for_username(username: &str) -> Result<Vec<u8>> {
    //let url_base = String::from("https://amax-emu.com/api");

    //let user_pfp_url = format!("{url_base}/profile/{username}/pfp.png");
    let user_pfp_url = String::from("https://cdn.discordapp.com/avatars/925665499692544040/483eb1b92db6a449a0e2bed9a8b48bb3.png");
    match get_image_from_url(user_pfp_url) {
        Ok(raw_bmp_data) => Ok(raw_bmp_data),
        Err(e) => Err(anyhow!(e)),
    }
}
