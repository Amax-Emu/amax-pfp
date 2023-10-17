use std::io::Read;

use std::{
    ffi::{c_void, CString},
    iter, mem, ptr,
};

use log::info;
use simplelog::*;

use windows::{
    core::{HRESULT, PCSTR, PCWSTR},
    Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader::{GetModuleHandleA, GetModuleHandleW, GetProcAddress},
    },
};

mod d3d9_utils;
//mod img_preprocess;
mod gamer_picture_manager;
mod img_preprocess;

pub static EXE_BASE_ADDR: i32 = 0x00400000;

use crate::gamer_picture_manager::*;

/// Called when the DLL is attached to the process.

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
            std::fs::File::create(".\\amax-pfp.log")
                .expect("Couldn't create log file: .\\amax-pfp.log"),
        ),
    ])
    .unwrap();
    log_panics::init();
    log::info!("Hi from: {module:X?}");

    unsafe {
        create_get_primary_profile_picture_hook();
    }

    let _ptr_base: *mut c_void = unsafe { GetModuleHandleA(PCSTR::null()) }.unwrap().0 as _;
}

pub fn free(module: HMODULE) {
    log::info!("Bye from: {module:X?}");
}
