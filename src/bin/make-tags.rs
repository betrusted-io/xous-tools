extern crate bootloader;

use std::fs::File;

use bootloader::xous_arguments::{XousArguments, XousSize};
use bootloader::memory::{MemoryRegion, MemoryRegions};
use bootloader::pid1::PID1;
use bootloader::xkrn::XousKernel;

const RAM_START: XousSize = 0x40000000;
const RAM_SIZE: XousSize = 16_777_216;
const FLASH_START: XousSize = 0x20000000;
const FLASH_SIZE: XousSize = 128 * 1024 * 1024;
const IO_START: XousSize = 0xe0000000;
const IO_SIZE: XousSize = 65_536;
const LCD_START: XousSize = 0xB0000000;
const LCD_SIZE: XousSize = 32_768;

fn main() {
    let mut args = XousArguments::new();

    let mut regions = MemoryRegions::new();
    regions.add(MemoryRegion::new(RAM_START, RAM_SIZE, "sram"));
    regions.add(MemoryRegion::new(FLASH_START, FLASH_SIZE, "ospi"));
    regions.add(MemoryRegion::new(IO_START, IO_SIZE, "ioio"));
    regions.add(MemoryRegion::new(LCD_START, LCD_SIZE, "mlcd"));

    let pid1 = PID1::new(0x20500000, 131072, 0x10000000,
        0x20000000, 32768,
        0x10000000,
        0xc0000000
    );

    let xkrn = XousKernel::new(0x20500000, 65536, 0x02000000,
        0x04000000, 32768,
        0x02000000);

    args.add(&regions);
    args.add(&pid1);
    args.add(&xkrn);

    println!("Arguments: {}", args);

    let f = File::create("args.bin").expect("Couldn't create args.bin");
    args.write(f).expect("Couldn't write to args");
}
