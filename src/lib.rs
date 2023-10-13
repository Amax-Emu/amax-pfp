use std::{ffi::{c_void, CString}, mem, iter, ptr};

use log::info;
use simplelog::*;
use winapi::shared::ntdef::LPCSTR;
use windows::{
    core::{PCSTR, PCWSTR, HRESULT},
    Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    Win32::{Foundation::HMODULE, System::LibraryLoader::{GetModuleHandleA, GetModuleHandleW, GetProcAddress}},
};

use windows::Win32::Graphics::Direct3D9::IDirect3D9;
use windows::Win32::Graphics::Direct3D9::IDirect3DDevice9;
use windows::Win32::Graphics::Direct3D9::*;

use retour::static_detour;

static_detour! {
    static GetPrimaryProfilePictureHook: unsafe extern "system" fn(u32) -> bool;
  }

static_detour! {
    static GamePictureManager_CreateHook: unsafe extern "system" fn(i32,i32,*const [u8;32],bool) -> bool;
}



/// Called when the DLL is attached to the process.
unsafe fn main() {

    // let address = 0x00d5e170;
    // let target = mem::transmute(address);
    // GetPrimaryProfilePictureHook
    // .initialize(target, primary_picture_load).unwrap()
    // .enable().unwrap();


    // let address = 0x0079dc50; //gamerpicmanager_create
    // let target = mem::transmute(address);
    // GamePictureManager_CreateHook
    // .initialize(target, manager_create).unwrap()
    // .enable().unwrap();

  }

/*
00000040 A8 EA 00 00:00 00 00 00|00 00 00 00:00 00 00 00
00000050 00 00 00 0C:00 00 00 00|01 47 41 4D:45 52 50 49
00000060 43 5F 30 00:00 00 00 00|00 00 00 00:00 00 00 00
00000070 00 00 00 00:00 00 00 00|00 00 00 00:40 00 00 00
00000080 00 04 00 E0:71 90 14 B0|CB 40 0F 00:00 00 00 4C


const DATA: [u8; 80] = [
	// Offset 0x00000040 to 0x0000008F
	0xA8, 0xEA, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x00,
	0x01, 0x47, 0x41, 0x4D, 0x45, 0x52, 0x50, 0x49, 0x43, 0x5F, 0x30, 0x00,
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x40, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0xE0, 0x71, 0x90, 0x14, 0xB0,
	0xCB, 0x40, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x4C
];


*/
struct C_GamerPicture { //total size on pc: 80
    unk0: u32, //0xA8, 0xEA, 0x00, 0x00
    Ref_aka_pad_id: u8, //0x00
    UserInformation: [u8;18], // 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00
    active: bool, // 0x00
    free: bool, // 0x01
    GamerPicName: [u8;20], //GAMERPIC_X or REMOTE_GAMERPIC_X
    size_as_big_end_temp: u32, // 0x00, 0x00, 0x00, 0x40
    unk_zeroes: u32, // 0x00, 0x00, 0x00, 0x00
    unk_4_as_u16: u16, //0x04, 0x00,
    new_texture_ptr: u32,  //0xE0, 0x71 0x90, 0x14
    default_texture_ptr: u32, //   0xB0, 0xCB 0x40, 0x0F
    unk4: u16, // 0x00, 0x00
    last_3_bytes: [u8;3] //0x00, 0x00, 0x4C
}

fn manager_create(max_local:i32,max_remote:i32,default_texture:*const [u8;32],small:bool) -> bool {
// Call the original `MessageBoxW`, but replace the caption
info!("max_local: {max_local}, max_remote:{max_remote},default_texture: {default_texture:?},small;:{small} ");
return true
}

fn primary_picture_load(pad_id:u32) -> bool {

  println!("{:?}",pad_id);


 

  return false
}

#[no_mangle]
#[allow(non_snake_case)]
extern "system" fn DllMain(
    dll_module: windows::Win32::Foundation::HMODULE,
    call_reason: u32,
    _reserved: *mut std::ffi::c_void,
) -> i32 {
    match call_reason {
        DLL_PROCESS_ATTACH => init(dll_module),
        DLL_PROCESS_DETACH => free(dll_module),
        _ => (),
    }
    true.into()
}

pub fn init(module: HMODULE) {
    let cfg = ConfigBuilder::new()
        .set_time_offset_to_local()
        .unwrap()
        .build();

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Trace,
            cfg,
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Trace,
            Config::default(),
            std::fs::File::create(".\\amax-pfp.log").expect("Couldn't create log file: .\\amax-pfp.log"),
        ),
    ])
    .unwrap();
    log_panics::init();
    log::info!("Hi from: {module:X?}");
    //unsafe { main(); }
   
    std::thread::spawn(|| {
        loop {
            unsafe {
                let mut new_gpu: *mut IDirect3DDevice9 = ptr::null_mut();
                // let EXE_BASE_ADDR = 0x00400000;
                // let mut addr = EXE_BASE_ADDR + 0x00D44EE4;
                let EXE_BASE_ADDR = 0x00400000;
    
                let start = EXE_BASE_ADDR + 0x00D44EE4;
    
    
                let ptr =  start as *const i32;
                info!("Addr of start: {:?}",start);
                info!("Addr of ptr1: {:p},value: {}",ptr,*ptr);
    
                if *ptr == 0 {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
    
                let step2 = *ptr;
    
                let step3 = step2 + 0x14;
                
                let step4 = step3 as *const i32;
                info!("Addr of step4: {:p},value: {}",step4,*step4);
                let d3d9_ptr_real = *step4 as *mut IDirect3DDevice9;
                info!("Addr of d3d device_real: {:p}",d3d9_ptr_real);
    
                // let step4 = step3 as *const i32;
                // info!("Addr of step4: {:p},value: {}",step4,*step4);
                let d3d9_ptr = step3 as *mut IDirect3DDevice9;
                info!("Addr of d3d device: {:p}",d3d9_ptr);
    
                // let mut addr2 = addr + 0x14;
                // info!("Addr of d3d device 2: {:?}",addr2);
                // let mut addr3 = &addr2 as *const i32;
                // info!("Addr of d3d device 3: {}",*addr3);
                
    
                // let value  = std::slice::from_raw_parts(addr3,4);
                // info!("Addr of d3d device 4: {:?}",value);
                // new_gpu = mem::transmute(addr);
        
                // new_gpu::Cre
        
                //info!("Addr of d3d device 2: {:?},{:?}",new_gpu,*new_gpu);
                let mut text: Option<IDirect3DTexture9> = None;
                info!("Addr of texture: {:p}",ptr::addr_of_mut!(text));
                let result = IDirect3DDevice9::CreateTexture( &*d3d9_ptr,64, 64, 1, 0, D3DFORMAT(827611204), D3DPOOL(1), ptr::addr_of_mut!(text), ptr::null_mut());
                info!("Result: {:?}",result);
    
    
                let address = get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileA")
                    .expect("could not find 'D3DXCreateTextureFromFileA' address");
                info!("Addr of D3DXCreateTextureFromFileA: {}",address);
    
    
                let filename = String::from("./test.bmp");
                let filename_bytes = filename.as_bytes().to_owned();
                type D3DXCreateTextureFromFileA = extern "stdcall" fn(device:&IDirect3DDevice9,filename:*const u8,text:*mut IDirect3DTexture9) -> HRESULT;
                
    
                let mut text2: IDirect3DTexture9 = text.unwrap();
    
                let my_func: D3DXCreateTextureFromFileA = std::mem::transmute(address);
    
                // let result = my_func(
                //     &*d3d9_ptr,
                //     ptr::addr_of!(filename_bytes[0]),
                //     ptr::addr_of_mut!(text)
                // );
    
                let result = my_func(
                    &*d3d9_ptr_real,
                    ptr::addr_of!(filename_bytes[0]),
                    ptr::addr_of_mut!(text2)
                );
                info!("Addr of texture: {:p}",ptr::addr_of_mut!(text2));
                let hook1 = ptr::addr_of_mut!(text2) as *mut i32;
                info!("REAL Addr of texture: {:?}",*hook1);
                info!("Result: {:?}",result);
    
                
                
                loop {
                    //Abomination to keep memory. Should be changed to static/box/update pfp own texture
                    std::thread::sleep(std::time::Duration::from_secs(60));
                }
            }
        }
    });

    let _ptr_base: *mut c_void = unsafe { GetModuleHandleA(PCSTR::null()) }.unwrap().0 as _;
}

pub fn free(module: HMODULE) {
    log::info!("Bye from: {module:X?}");
}


fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
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