#![feature(ptr_from_ref)]
#![no_std]

extern crate spin; // we need a mutex in devices::cga_print
extern crate rlibc;
extern crate tinyrlibc;
extern crate alloc;

use alloc::format;
use alloc::string::ToString;
use core::mem::size_of;
use core::panic::PanicInfo;
use chrono::DateTime;
use linked_list_allocator::LockedHeap;
use multiboot2::{BootInformation, BootInformationHeader, Tag};
use multiboot2::MemoryAreaType::{Available};
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use pc_keyboard::layouts::{AnyLayout, De105Key};
use crate::device::{cpu, pic, pit, ps2};
use crate::kernel::int_disp;

// insert other modules
mod device;
mod kernel;
#[macro_use]
mod library;
mod consts;

use crate::library::graphic::lfb_terminal;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe { lfb_terminal::WRITER.force_unlock(); };
    println!("Panic: {}", info);
    loop {}
}

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[no_mangle]
pub unsafe extern fn startup(mbi: u64) {
    // Get multiboot information
    let multiboot = BootInformation::load(mbi as *const BootInformationHeader).unwrap();

    // Initialize memory management
    let memory_info = multiboot.memory_map_tag().unwrap();
    let mut heap_area = memory_info.memory_areas().get(0).unwrap();

    for area in memory_info.memory_areas() {
        if area.typ() == Available && area.size() > heap_area.size() {
            heap_area = area;
        }
    }

    // Initialize memory allocation
    ALLOCATOR.lock().init(heap_area.start_address() as *mut u8, (heap_area.end_address() - heap_area.start_address()) as usize);

    // Initialize interrupts
    pic::init();
    int_disp::init();
    cpu::enable_int();

    // Initialize timer;
    pit::init();
    pit::plugin();

    // Initialize framebuffer
    let fb_info = multiboot.framebuffer_tag().unwrap().unwrap();
    lfb_terminal::initialize(fb_info.address() as * mut u8, fb_info.pitch(), fb_info.width(), fb_info.height(), fb_info.bpp());

    let version = format!("v{} ({})", built_info::PKG_VERSION, built_info::PROFILE);
    let date = match DateTime::parse_from_rfc2822(built_info::BUILT_TIME_UTC) {
        Ok(date_time) => date_time.format("%Y-%m-%d %H:%M:%S").to_string(),
        Err(_) => "Unknown".to_string()
    };
    let git_ref = match built_info::GIT_HEAD_REF {
        Some(str) => str,
        None => "Unknown"
    };
    let git_commit = match built_info::GIT_COMMIT_HASH_SHORT {
        Some(str) => str,
        None => "Unknown"
    };
    let bootloader_name = match multiboot.boot_loader_name_tag() {
        Some(tag) => if tag.name().is_ok() { tag.name().unwrap() } else { "Unknown" },
        None => "Unknown"
    };

    println!(include_str!("banner.txt"), version, date, git_ref, git_commit, bootloader_name);

    // Initialize ACPI tables
    let rsdp_addr: usize = if let Some(rsdp_tag) = multiboot.rsdp_v2_tag() {
        core::ptr::from_ref(rsdp_tag) as usize + size_of::<Tag>()
    } else if let Some(rsdp_tag) = multiboot.rsdp_v1_tag() {
        core::ptr::from_ref(rsdp_tag) as usize + size_of::<Tag>()
    } else {
        panic!("ACPI not available!");
    };

    kernel::acpi::init(rsdp_addr);
    let tables = kernel::acpi::get_tables();
    println!("ACPI revision: {}", tables.revision());

    // Initialize keyboard
    ps2::init_controller();
    ps2::init_keyboard();
    ps2::plugin_keyboard();

    let mut decoder = Keyboard::new(ScancodeSet1::new(), AnyLayout::De105Key(De105Key), HandleControl::Ignore);
    println!("Boot time: {} ms", pit::get_systime_ms());

    loop {
        let scancode = ps2::get_keyboard().lock().pop_scancode();
        if let Ok(Some(event)) = decoder.add_byte(scancode) {
            if let Some(key) = decoder.process_keyevent(event) {
                match key {
                    DecodedKey::Unicode(c) => {
                        print!("{}", c);
                    }
                    _ => {}
                }
            }
        }
    }
}

