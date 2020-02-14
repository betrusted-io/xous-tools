extern crate bootloader;
extern crate xmas_elf;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use xmas_elf::sections::ShType;
use xmas_elf::program::Type as ProgramType;
use xmas_elf::ElfFile;

fn main() {
    let mut s = Vec::new();
    let mut size = 0;
    {
        let mut fi = File::open("../kernel/target/riscv32i-unknown-none-elf/debug/xous-kernel")
            .expect("Couldn't open kernel");
        fi.read_to_end(&mut s).expect("Couldn't read elf file");
    }
    let mut f = File::create("init.bin").expect("Couldn't create init.bin");
    let elf = ElfFile::new(&s).expect("Couldn't parse elf file");
    println!("Entrypoint: {:08x}", elf.header.pt2.entry_point());

    let mut expected_size = 0;
    let mut program_offset = 0;

    for ph in elf.program_iter() {
        println!("Program Header: {:?}", ph);
        if ph.get_type() == Ok(ProgramType::Load) {
            expected_size = ph.file_size();
            program_offset = ph.offset();
        }
        println!("Physical address: {:08x}", ph.physical_addr());
        println!("Virtual address: {:08x}", ph.virtual_addr());
    }
    println!("File should be {} bytes, and program starts at 0x{:x}", expected_size, program_offset);
    for s in elf.section_iter() {
        let name = s.get_name(&elf).unwrap_or("<<error>>");
        if s.get_type() == Ok(ShType::NoBits) {
            println!("(Skipping section {} -- invalid type)", name);
            continue;
        }

        if s.address() == 0 {
            println!("(Skipping section {} -- invalid address)", name);
            continue;
        }

        println!("Section {}:", name);
        println!("    flags:            {:?}", s.flags());
        println!("    type:             {:?}", s.get_type());
        println!("    address:          {:08x}", s.address());
        println!("    offset:           {:08x}", s.offset());
        println!("    size:             {:?}", s.size());
        println!("    link:             {:?}", s.link());
        size += s.size();
        if size & 3 != 0 {
            println!("Size is not padded!");
        }
        f.seek(SeekFrom::Start(s.offset() - program_offset)).expect("Couldn't seek file");
        let section_data = s.raw_data(&elf);
        f.write(section_data).expect("Couldn't save data");
    }
    let observed_size = f.seek(SeekFrom::End(0)).expect("Couldn't seek to end of file");
    if expected_size != observed_size {
        panic!("Expected to read {} bytes, but actually read {} bytes", expected_size, observed_size);
    }

    println!("Copied {} bytes of data", observed_size);
}
