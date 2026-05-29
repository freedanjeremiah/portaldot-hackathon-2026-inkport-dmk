#![no_std]
#![no_main]
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! { core::arch::wasm32::unreachable() }

// pallet-contracts seal0 host ABI (Substrate ~2021 era)
#[link(wasm_import_module = "seal0")]
extern "C" {
    fn seal_input(buf_ptr: *mut u8, buf_len_ptr: *mut u32);
    fn seal_return(flags: u32, data_ptr: *const u8, data_len: u32);
    fn seal_get_storage(key_ptr: *const u8, out_ptr: *mut u8, out_len_ptr: *mut u32) -> u32;
    fn seal_set_storage(key_ptr: *const u8, value_ptr: *const u8, value_len: u32);
}

// fixed 32-byte storage key holding a little-endian u32 counter
static KEY: [u8; 32] = [0u8; 32];

fn store(val: u32) {
    let bytes = val.to_le_bytes();
    unsafe { seal_set_storage(KEY.as_ptr(), bytes.as_ptr(), 4); }
}

fn load() -> u32 {
    let mut buf = [0u8; 4];
    let mut len: u32 = 4;
    let rc = unsafe { seal_get_storage(KEY.as_ptr(), buf.as_mut_ptr(), &mut len as *mut u32) };
    if rc == 0 && len == 4 { u32::from_le_bytes(buf) } else { 0 }
}

// constructor: counter = 0
#[no_mangle]
pub extern "C" fn deploy() {
    store(0);
}

// message dispatch by 4-byte selector
//   0x00000001 = inc   (state-changing)
//   0x00000002 = get   (returns the u32 LE)
#[no_mangle]
pub extern "C" fn call() {
    let mut input = [0u8; 16];
    let mut in_len: u32 = 16;
    unsafe { seal_input(input.as_mut_ptr(), &mut in_len as *mut u32); }
    let sel = [input[0], input[1], input[2], input[3]];
    let val = load();
    match sel {
        [0, 0, 0, 1] => { store(val + 1); unsafe { seal_return(0, core::ptr::null(), 0); } }
        [0, 0, 0, 2] => { let out = val.to_le_bytes(); unsafe { seal_return(0, out.as_ptr(), 4); } }
        _            => { unsafe { seal_return(1, core::ptr::null(), 0); } }
    }
}
