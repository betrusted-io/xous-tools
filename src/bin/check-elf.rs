// extern crate bootloader;
// extern crate xmas_elf;
extern crate xous_tools;
use std::fs::File;
use std::io::Write;
use xous_tools::elf;

fn main() {
    let pd = elf::read_program("../kernel/target/riscv32i-unknown-none-elf/debug/xous-kernel")
        .expect("Couldn't read program");
    let mut f = File::create("output.bin").expect("Couldn't create output.bin");
    f.write_all(&pd.program)
        .expect("Couldn't write to output.bin");

    println!("Data offset: {:08x}", pd.data_offset);
    println!("Data size: {}", pd.data_size);
    println!("Text offset: {:08x}", pd.text_offset);
    println!("Entrypoint: {:08x}", pd.entry_point);
    println!("Copied {} bytes of data", pd.program.len());
}
