#![no_main]
#![no_std]

extern crate alloc;
use core::ffi::CStr;

use alloc::{borrow::ToOwned, format, string::String, vec, vec::Vec};
use molecule::prelude::Entity;
use spore_dob_1::decoder::{
    dobs_parse_parameters, dobs_parse_syscall_parameters,
    types::{DOB1Output, Image},
};

const HEAPS_SIZE: usize = 1024 * 1024 * 2; // 2M

static mut HEAPS: [u8; HEAPS_SIZE] = [0; HEAPS_SIZE];
#[global_allocator]
static ALLOC: linked_list_allocator::LockedHeap = linked_list_allocator::LockedHeap::empty();

#[panic_handler]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    // If the main thread panics it will terminate all your threads and end your program with code 101.
    // See: https://github.com/rust-lang/rust/blob/master/library/core/src/macros/panic.md
    syscall_write(&format!("{panic_info:?}").as_bytes().to_vec());
    syscall_exit(101)
}

fn syscall(mut a0: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64, a7: u64) -> u64 {
    unsafe {
        core::arch::asm!(
          "ecall",
          inout("a0") a0,
          in("a1") a1,
          in("a2") a2,
          in("a3") a3,
          in("a4") a4,
          in("a5") a5,
          in("a6") a6,
          in("a7") a7
        )
    }
    a0
}

fn syscall_exit(code: u64) -> ! {
    syscall(code, 0, 0, 0, 0, 0, 0, 93);
    loop {}
}

fn syscall_write(buf: &Vec<u8>) -> u64 {
    syscall(buf.as_ptr() as *const u8 as u64, 0, 0, 0, 0, 0, 0, 2177)
}

fn syscall_combine_image(buf: &mut Vec<u8>, buf_size: &mut u64, molecule_bytes: &[u8]) -> u64 {
    syscall(
        buf.as_mut_ptr() as *mut u8 as u64,
        buf_size as *mut u64 as u64,
        molecule_bytes.as_ptr() as *const u8 as u64,
        molecule_bytes.len() as u64,
        0,
        0,
        0,
        2077,
    )
}

#[no_mangle]
pub unsafe extern "C" fn _start() {
    core::arch::asm!(
        "lw a0,0(sp)", // Argc.
        "add a1,sp,8", // Argv.
        "li a2,0",     // Envp.
        "call main",
        "li a7, 93",
        "ecall",
    );
}

#[no_mangle]
unsafe extern "C" fn main(argc: u64, argv: *const *const i8) -> u64 {
    unsafe {
        ALLOC.lock().init(HEAPS.as_mut_ptr(), HEAPS_SIZE);
    }

    let mut args = Vec::new();
    for i in 0..argc {
        let argn = unsafe { CStr::from_ptr(argv.add(i as usize).read()) };
        args.push(argn.to_bytes());
    }
    let dob_params = match dobs_parse_parameters(args) {
        Ok(value) => value,
        Err(err) => return err as u64,
    };
    let patterns = match dobs_parse_syscall_parameters(&dob_params) {
        Ok(value) => value,
        Err(err) => return err as u64,
    };
    let images = patterns
        .into_iter()
        .map(|(name, pattern)| {
            let mut buffer = vec![];
            let mut buffer_size = 0u64;
            syscall_combine_image(&mut buffer, &mut buffer_size, pattern.as_slice()); // determine real buffer size
            buffer.resize(buffer_size as usize, 0);
            syscall_combine_image(&mut buffer, &mut buffer_size, pattern.as_slice()); // fill buffer
            let base64_image = String::from_utf8(buffer).expect("Invalid UTF-8 sequence");
            Image {
                name,
                type_: "image/png;base64".to_owned(),
                content: base64_image,
            }
        })
        .collect::<Vec<_>>();

    let dob1_output = DOB1Output {
        traits: dob_params.dob0_output,
        images,
    };
    let mut output = serde_json::to_string(&dob1_output)
        .expect("Failed to serialize output")
        .as_bytes()
        .to_vec();
    output.push(0);
    syscall_write(&output);
    0
}

// #[no_mangle]
// unsafe extern "C" fn main(argc: u64, argv: *const *const i8) -> u64 {
//     use spore_dob_1::generated;

//     unsafe {
//         ALLOC.lock().init(HEAPS.as_mut_ptr(), HEAPS_SIZE);
//     }

//     let mut buffer = vec![0u8; 1024 * 1024];
//     let mut buffer_size = 0u64;
//     let color = generated::Color::new_builder()
//         .set(b"#FF0000".map(molecule::prelude::Byte::new).to_vec())
//         .build();
//     let uri = generated::URI::new_builder()
//         .set(
//             b"btcfs://b2f4560f17679d3e3fca66209ac425c660d28a252ef72444c3325c6eb0364393i0"
//                 .map(molecule::prelude::Byte::new)
//                 .to_vec(),
//         )
//         .build();
//     let color_item = generated::Item::new_builder().set(color).build();
//     let uri_item = generated::Item::new_builder().set(uri).build();
//     let pattern = generated::ItemVec::new_builder()
//         .push(color_item)
//         .push(uri_item)
//         .build();
//     syscall_combine_image(&mut buffer, &mut buffer_size, pattern.as_slice());

//     syscall_write(
//         &format!("final image size: {}\0", buffer_size)
//             .as_bytes()
//             .to_vec(),
//     );
//     0
// }
