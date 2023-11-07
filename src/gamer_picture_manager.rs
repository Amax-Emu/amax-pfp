use crate::img_preprocess::get_image_from_url;
use anyhow::anyhow;
use anyhow::Result;
use log::debug;
use log::warn;

use fxhash::FxHashMap;
use log::{error, info};
use retour::static_detour;
use std::thread;
use std::time::Duration;
use std::{fs, io, mem, ptr, str::Utf8Error};
use widestring::WideCString;
use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;
extern crate fxhash;

pub static GAMER_PICTURE_MANAGER: i32 = 0x011a89c8;
/*
Due to how memory in Blur works there are some static locations in memory, that contain pointers to some structures.
This one points to GAMER_PICTURE_MANAGER, which is great.
*/

pub static _FRIEND_LIST: i32 = 0x011C7040;
/*
This one points to your friend list.
*/

//static URL_BASE: String = String::from("https://amax-emu.com/api");

//static PFP_CACHE: HashMap<String, Vec<u8>> = Lazy::new(||HashMap::new());

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct GamerPictureManager {
    thread: [u8; 20], // C8 AA EA 00 00 00 00 00 00 00 00 00 0D F0 AD BA 0D F0 AD BA || This is a thread pointer at PS3. I don't know what is it on PC.
    local_pictures_ptr: *const [*mut C_GamerPicture; 4],
    local_pictures_size: usize,
    local_pictures_len: usize, //this one is used in GetTotalPicturesFunctions
    remote_pictures_ptr: *const [*mut C_GamerPicture; 19], //hardcoding to 19
    remote_pictures_size: usize, // Has value of 26. Bug? Cut feature? We will never know.
    remoter_pictures_len: usize, //this one is used in GetTotalPicturesFunctions
}

#[derive(Debug)]
#[repr(C)]
struct C_GamerPicture {
    //total size on pc: 80
    unk_ptr0: u32, //0x4C 0xA8, 0xEA, 0x00,
    ref1: u16,
    user_dw_id: u64,
    user_information: [u8; 8], // 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00
    active: bool,              // 0x00
    free: bool,                // 0x01
    gamer_pic_name: [u8; 30],  //GAMERPIC_X or REMOTE_GAMERPIC_X
    size_as_big_end_temp: u32, // 0x00, 0x00, 0x00, 0x00
    unk_zeroes: u32,           // 0x00, 0x40 0x00, 0x00,
    unk_4_as_u16: u16,         //0x04, 0x00,
    texture_ptr: IDirect3DTexture9, //0xE0, 0x71 0x90, 0x14
    default_texture_ptr: u32,  //   0xB0, 0xCB 0x40, 0x0F
    unk4: u32,                 // 0x00, 0x00
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct NetPlayer {
    // this structure contain a lot of usefull data, but we're not interested
    unk0: [u8; 72],
    user_dw_id: u64,
    zeroes: [u8; 8],
    username_in_utf_16: [u16; 16],
    unk1: [u8; 164],
    mp_lobby_ref_id: u8, //position of user in mplobby and the exact value remote_picture ref should be set to
    unk2: [u8; 107],
}

#[derive(Debug)]
#[repr(C)]
struct MpUiLobbyData {
    // A rather short, but very usefull struct.
    unk0: [u8; 28],
    net_players: *mut [NetPlayer; 19],
    total_players: u32, //client itself also counts, so to get number of connected remote users substract 1
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

//0079da10
static_detour! {
    static GamePictureManager_WipeRemotePictures: unsafe extern "fastcall" fn(*mut GamerPictureManager);
}
//little pesky function messing up things

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
    GetPrimaryProfilePictureHook
        .initialize(target, primary_picture_load)
        .unwrap()
        .enable()
        .unwrap();
}

pub unsafe fn create_wipe_remote_pictures_hook() {
    let address = 0x0079da10;
    let target = mem::transmute(address);
    GamePictureManager_WipeRemotePictures
        .initialize(target, wipe_remote_pictures)
        .unwrap()
        .enable()
        .unwrap();
}

fn request_remote_picture(unk1: i32) -> bool {
    info!("unk1: {unk1}");
    return true;
}

fn wipe_remote_pictures(gpm: *mut GamerPictureManager) {
    info!("Not wiping pictures!");
    return;
}

fn manager_create(
    max_local: i32,
    max_remote: i32,
    default_texture: *const [u8; 32],
    small: bool,
) -> bool {
    debug!("max_local: {max_local}, max_remote:{max_remote},default_texture: {default_texture:?},small;:{small} ");
    return true;
}

unsafe fn get_gamer_picture_manager() -> GamerPictureManager {
    let local_start = GAMER_PICTURE_MANAGER;
    debug!("Addr of start: {:?}", local_start);

    let ptr = local_start as *const i32;
    let ptr = *ptr as *mut GamerPictureManager;
    debug!("Addr of GamerPictureManager ptr: {:p}", &ptr);
    //todo: there could be cases were GPM wouldn't be initiated.
    let gpm = *ptr;

    return gpm;
}

fn pretty_name(name_buf: &[u8]) -> String {
    let name = String::from_utf8(name_buf.to_vec()).unwrap();
    return name.trim_matches(char::from(0)).to_string();
}

fn primary_picture_load() -> bool {
    info!("GetPrimaryProfilePicture hook");
    unsafe {
        let gamer_picture_manager = get_gamer_picture_manager();

        for picture in *gamer_picture_manager.local_pictures_ptr {
            debug!(
                "Addr of picture: {:p} Data: {:?}",
                ptr::addr_of!(picture),
                picture
            );

            let name = pretty_name(&(*picture).gamer_pic_name);
            debug!("Processing {}", &name);

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
                    ptr::addr_of_mut!((*picture).texture_ptr),
                    img_data,
                    64,
                    64,
                );

                debug!("Result: {:?}", result);

                if result.is_err() {
                    panic!();
                }

                (*picture).active = true;
            }
        }
    }

    return false;
}

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
    let url_base = String::from("https://amax-emu.com/api");
    let user_pfp_url = format!("{url_base}/players/pfp/name/{username}");

    //let user_pfp_url = String::from("https://cdn.discordapp.com/avatars/925665499692544040/483eb1b92db6a449a0e2bed9a8b48bb3.png");
    //let user_pfp_url = "https://cs.amax-emu.com/amax_logo.png".to_string();
    //^ A huge veriety of options for Strings creating! Crazy!

    match get_image_from_url(user_pfp_url) {
        Ok(raw_bmp_data) => Ok(raw_bmp_data),
        Err(e) => Err(anyhow!(e)),
    }
}

pub unsafe fn remote_pfp_updater() {
    let mut pfp_cache: FxHashMap<u64, Vec<u8>> = FxHashMap::default();

    loop {
        let start = crate::EXE_BASE_ADDR + 0x00DB4530;
        let ptr = start as *const i32;

        if *ptr == 0 {
            warn!("mp_ui_lobby_data pointer is empty! Sleeping");
            thread::sleep(Duration::from_secs(1));
            continue;
        }

        let mp_ui_lobby_data_ptr = *ptr as *mut MpUiLobbyData;
        let mp_ui_lobby_data = &*mp_ui_lobby_data_ptr;

        if mp_ui_lobby_data.total_players < 2 {
            debug!(
                "We're alone in the lobby ({}), skipping... ({:?})",
                mp_ui_lobby_data.total_players,
                ptr::addr_of!(mp_ui_lobby_data.total_players)
            );
            thread::sleep(Duration::from_secs(1));
            continue;
        }

        debug!(
            "We're not alone! Doing magic. {}. {:?}",
            mp_ui_lobby_data.total_players,
            ptr::addr_of!(mp_ui_lobby_data.total_players)
        );

        let gamer_picture_manager = get_gamer_picture_manager();
        debug!("Got gamer_picture_manager");
        let remote_pictures = *gamer_picture_manager.remote_pictures_ptr;
        debug!("Got Remote pictures");
        let ui_players = mp_ui_lobby_data.net_players.read();
        debug!("Got network players");

        for player_lobby_num in 0..(mp_ui_lobby_data.total_players - 1) {
            let player = ui_players[player_lobby_num as usize];
            let pretty_name = WideCString::from_vec_truncate(player.username_in_utf_16)
                .to_string()
                .unwrap();
            debug!("Pretty name: {pretty_name}");

            let picture = remote_pictures[player_lobby_num as usize];

            if (*picture).user_dw_id == player.user_dw_id {
                debug!("PFP for this user ({}) is already set", pretty_name);
                if (*picture).free == false {
                    //failsafe
                    (*picture).ref1 = player.mp_lobby_ref_id as u16;
                }
                continue;
            }

            if pfp_cache.contains_key(&player.user_dw_id) {
                let img_data = pfp_cache.get(&player.user_dw_id).unwrap().clone();

                let _result = crate::d3d9_utils::d3d9_load_texture_from_memory_ex(
                    ptr::addr_of_mut!((*picture).texture_ptr),
                    img_data,
                    64,
                    64,
                );

                (*picture).user_dw_id = player.user_dw_id;
                (*picture).ref1 = player.mp_lobby_ref_id as u16;
                (*picture).active = true;
                (*picture).free = false;

                let _res = trigger_lobby_update();

            } else {

                let url_base = String::from("https://amax-emu.com/api");
                let user_pfp_url = format!("{url_base}/players/pfp/name/{pretty_name}");

                match get_image_from_url(user_pfp_url) {
                    Ok(img_data_url) => {
                        let img_data = img_data_url.clone();
                        pfp_cache.insert(player.user_dw_id, img_data_url.clone());

                        let _result = crate::d3d9_utils::d3d9_load_texture_from_memory_ex(
                            ptr::addr_of_mut!((*picture).texture_ptr),
                            img_data,
                            64,
                            64,
                        );

                        (*picture).user_dw_id = player.user_dw_id;
                        (*picture).ref1 = player.mp_lobby_ref_id as u16;
                        (*picture).active = true;
                        (*picture).free = false;

                        let _res = trigger_lobby_update();
                    }
                    Err(e) => {
                        error!("Failed to retrive image: {e}");
                    }
                };
            }
        }

        thread::sleep(Duration::from_millis(1000));
    }
}

pub unsafe fn trigger_lobby_update() -> Result<(), ()> {
    //the final piece of the puzzle
    debug!("Triggering lobby update");
    let start = crate::EXE_BASE_ADDR + 0x00E42FF8;

    let ptr = start as *const i32;
    debug!("Addr of ptr1: {:p},value: 0x0{:X}", ptr, *ptr);

    if *ptr == 0 {
        return Err(());
    };

    let step2 = *ptr;

    let step3 = step2 + 0x181;

    let lobby_need_update = step3 as *mut bool;
    *lobby_need_update = true;
    Ok(())
}
