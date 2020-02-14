extern crate bootloader;
extern crate xmas_elf;
#[macro_use]
extern crate clap;

use std::fs::File;

use bootloader::tags::init::Init;
use bootloader::tags::memory::{MemoryRegion, MemoryRegions};
use bootloader::tags::xkrn::XousKernel;
use bootloader::utils::{parse_csr_csv, parse_u32};
use bootloader::xous_arguments::XousArguments;

use clap::{App, Arg};

fn main() {
    let matches = App::new("Xous Image Creator")
        .version(crate_version!())
        .author("Sean Cross <sean@xobs.io>")
        .about("Create a boot image for Xous")
        .arg(
            Arg::with_name("kernel")
                .short("k")
                .long("kernel")
                .value_name("KERNEL_ELF")
                .takes_value(true)
                .required(true)
                .help("Kernel ELF image to bundle into the image"),
        )
        .arg(
            Arg::with_name("init")
                .short("i")
                .long("init")
                .takes_value(true)
                .multiple(true)
                .help("Initial program to load"),
        )
        .arg(
            Arg::with_name("csv")
                .short("c")
                .long("csv")
                .alias("csr-csv")
                .alias("csr")
                .value_name("CSR_CSV")
                .help("csr.csv file from litex")
                .takes_value(true)
                .required_unless("ram"),
        )
        .arg(
            Arg::with_name("ram")
                .short("r")
                .long("ram")
                .takes_value(true)
                .value_name("OFFSET:SIZE")
                .required_unless("csv")
                .help("RAM offset and size, in the form of [offset]:[size]"),
        )
        .get_matches();

    let mut ram_offset = Default::default();
    let mut ram_size = Default::default();
    let mut ram_name = "sram".to_owned();
    let mut regions = MemoryRegions::new();

    if let Some(val) = matches.value_of("ram") {
        let ram_parts: Vec<&str> = val.split(":").collect();
        if ram_parts.len() != 2 {
            eprintln!("Error: --ram argument should be of the form [offset]:[size]");
            return;
        }

        ram_offset = match parse_u32(ram_parts[0]) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("Error: Unable to parse {}: {:?}", ram_parts[0], e);
                return;
            }
        };

        ram_size = match parse_u32(ram_parts[1]) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("Error: Unable to parse {}: {:?}", ram_parts[1], e);
                return;
            }
        };
    }

    if let Some(csr_csv) = matches.value_of("csv") {
        let hv = parse_csr_csv(csr_csv).unwrap();
        let mut found_ram_name = None;

        // Look for the largest "ram" block, which we'll treat as main memory
        for (k, v) in &hv.regions {
            if k.find("ram").is_some() {
                if v.length > ram_size {
                    ram_size = v.length;
                    ram_offset = v.start;
                    found_ram_name = Some(k.clone());
                }
            }
        }

        if found_ram_name.is_none() {
            eprintln!("Error: Couldn't find a memory region named \"ram\" in csv file");
            return;
        }

        // Now that we know which block is ram, add the other regions.
        let found_ram_name = found_ram_name.unwrap();
        for (k, v) in &hv.regions {
            let mut region_name = k.clone();
            region_name.push_str("    ");
            region_name.truncate(4);
            if *k == found_ram_name {
                println!(
                    "Skipping ram block {} ({:08x} - {:08x})",
                    k,
                    ram_offset,
                    ram_offset + ram_size
                );
                ram_name = region_name.clone();
                continue;
            }
            regions.add(MemoryRegion::new(v.start, v.length, &region_name));
        }
    }

    let mut args = XousArguments::new(ram_offset, ram_size, &ram_name);

    if regions.len() > 0 {
        args.add(&regions);
    }

    let init = Init::new(
        0x20500000, 131072, 0x10000000, 0x20000000, 32768, 0x10000000, 0xc0000000,
    );
    args.add(&init);

    let xkrn = XousKernel::new(
        0x20500000, 65536, 0x02000000, 0x04000000, 32768, 0x02000000, 0x44320,
    );
    args.add(&xkrn);

    println!("Arguments: {}", args);

    let f = File::create("args.bin").expect("Couldn't create args.bin");
    args.write(f).expect("Couldn't write to args");
}
