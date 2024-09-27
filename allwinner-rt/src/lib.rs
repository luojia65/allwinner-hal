//! Allwinner D1 ROM runtime.
//!
//! # Usage
//!
//! Here's an sample usage of this crate:
//!
//! ```no_run
//! use allwinner_rt::{entry, Clocks, Peripherals};
//!
//! #[entry]
//! fn main(p: Peripherals, c: Clocks) {
//!     /* code */
//! }
//! ```
#![feature(naked_functions, asm_const)]
#![no_std]

#[macro_use]
mod macros;

#[cfg(any(feature = "nezha", feature = "lichee"))]
mod mctl;
#[cfg(any(feature = "nezha", feature = "lichee"))]
/// Dram initializing function.
pub use mctl::init as dram_init;

pub use allwinner_rt_macros::entry;

use core::arch::asm;

pub mod soc {
    pub mod d1;
}

/// eGON.BT0 identifying structure.
// TODO verify with original ROM source code
#[repr(C)]
pub struct EgonHead {
    pub magic: [u8; 8],
    pub checksum: u32,
    pub length: u32,
    pub pub_head_size: u32,
    pub pub_head_version: [u8; 4],
    pub return_addr: u32,
    pub run_addr: u32,
    pub boot_cpu: u32,
    pub platform: [u8; 8],
}

/// Jump over head data to executable code.
///
/// # Safety
///
/// Naked function.
///
/// NOTE: `mxstatus` is a custom T-Head register. Do not confuse with `mstatus`.
/// It allows for configuring special eXtensions. See further below for details.
#[naked]
#[link_section = ".text.entry"]
unsafe extern "C" fn start() -> ! {
    const STACK_SIZE: usize = 1024;
    #[link_section = ".bss.uninit"]
    static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
    asm!(
        // Enable T-Head ISA extension
        "li     t1, 1 << 22",
        "csrs   0x7C0, t1",
        // Invalidate instruction and data cache, branch history table
        // and branch target buffer table
        "li     t1, 0x30013",
        "csrs   0x7C2, t1",
        // Disable interrupt
        "csrw   mie, zero",
        // Prepare programming language stack
        "la     sp, {stack}
        li      t0, {stack_size}
        add     sp, sp, t0",
        // Clear `.bss` section
        "la     t1, sbss
        la      t2, ebss
    1:  bgeu    t1, t2, 1f
        sd      zero, 0(t1)
        addi    t1, t1, 8
        j       1b
    1:  ",
        // Prepare data segment
    "   la      t3, sidata
        la      t4, sdata
        la      t5, edata
    1:  bgeu    t4, t5, 2f
        ld      t6, 0(t3)
        sd      t6, 0(t4)
        addi    t3, t3, 8
        addi    t4, t4, 8
        j       1b",
    "2: ",
        // Start Rust main function
        "call   {main}",
        // Platform halt if main function returns
    "1: wfi
        j       1b",
        stack      =   sym STACK,
        stack_size = const STACK_SIZE,
        main       =   sym main,
        options(noreturn)
    )
}

#[rustfmt::skip]
extern "Rust" {
    // This symbol is generated by `#[entry]` macro in allwinner-rt or other ROM-stage software packages.
    fn main();
}

#[no_mangle]
#[link_section = ".head.egon"]
static EGON_HEAD: EgonHead = EgonHead {
    magic: *b"eGON.BT0",
    checksum: 0x5F0A6C39, // real checksum will be filled by blob generator
    length: 0x8000,
    pub_head_size: 0,
    pub_head_version: *b"3000",
    return_addr: 0,
    run_addr: 0,
    boot_cpu: 0,
    platform: *b"\0\03.0.0\0",
};

core::arch::global_asm! {
    ".section .text.head",
    "head_jump:",
    "j  {}",
    sym start,
}

#[cfg(any(feature = "nezha", feature = "lichee"))]
pub use {
    self::soc::d1::{Peripherals, __rom_init_params},
    allwinner_hal::ccu::Clocks,
};

#[cfg(not(any(feature = "nezha", feature = "lichee")))]
pub struct Peripherals {}
#[cfg(not(any(feature = "nezha", feature = "lichee")))]
pub struct Clocks {}
#[cfg(not(any(feature = "nezha", feature = "lichee")))]
#[doc(hidden)]
pub fn __rom_init_params() -> (Peripherals, Clocks) {
    (Peripherals {}, Clocks {})
}
