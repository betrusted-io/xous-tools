#[macro_use]
extern crate clap;

extern crate crc;

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

use xous_tools::elf::read_program;
use xous_tools::tags::init::Init;
use xous_tools::tags::memory::{MemoryRegion, MemoryRegions};
use xous_tools::tags::xkrn::XousKernel;
use xous_tools::tags::bflg::Bflg;
use xous_tools::utils::{parse_csr_csv, parse_u32};
use xous_tools::xous_arguments::XousArguments;

use clap::{App, Arg};

fn pad_file_to_4_bytes(f: &mut File) {
    while f
        .seek(SeekFrom::Current(0))
        .expect("couldn't check file position")
        & 3
        != 0
    {
        f.seek(SeekFrom::Current(1)).expect("couldn't pad file");
    }
}

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
                .number_of_values(1)
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
        .arg(
            Arg::with_name("debug")
                .short("d")
                .long("debug")
                .takes_value(false)
                .help("Reduce kernel-userspace security and enable debugging programs"),
        )
        .arg(
            Arg::with_name("output")
                .value_name("OUTPUT")
                .required(true)
                .help("Output file to store tag and init information"),
        )
        .get_matches();

    let mut ram_offset = Default::default();
    let mut ram_size = Default::default();
    let mut ram_name = MemoryRegion::make_name("sram");
    let mut regions = MemoryRegions::new();
    let mut memory_required = 0;

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

        memory_required += ram_size / 4096;
    }

    if let Some(csr_csv) = matches.value_of("csv") {
        let hv = parse_csr_csv(csr_csv).unwrap();
        let mut found_ram_name = None;
        fn round_mem(src: u32) -> u32 {
            (src + 4095) & !4095
        }
        // Look for the largest "ram" block, which we'll treat as main memory
        for (k, v) in &hv.regions {
            if k.find("ram").is_some() {
                if v.length > ram_size {
                    ram_size = round_mem(v.length);
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
        let found_ram_name = MemoryRegion::make_name(&found_ram_name.unwrap());
        for (k, v) in &hv.regions {
            memory_required += round_mem(v.length) / 4096;
            let region_name = MemoryRegion::make_name(k);
            // Don't add the RAM section to the extra regions block.
            if region_name == found_ram_name {
                ram_name = region_name;
                continue;
            }
            // Don't add empty sections.
            if round_mem(v.length) == 0 {
                continue;
            }
            regions.add(MemoryRegion::new(v.start, round_mem(v.length), region_name));
        }
    }

    let kernel = read_program(
        matches
            .value_of("kernel")
            .expect("kernel was somehow missing"),
    )
    .expect("unable to read kernel");
    let mut programs = vec![];
    if let Some(program_paths) = matches.values_of("init") {
        for program_path in program_paths {
            programs.push(
                read_program(program_path)
                    .expect(&format!("unable to read program {}", program_path)),
            )
        }
    }

    let mut args = XousArguments::new(ram_offset, ram_size, ram_name);

    if regions.len() > 0 {
        args.add(regions);
    }

    // Add tags for init and kernel.  These point to the actual data, which should
    // immediately follow the tags.  Therefore, we must know the length of the tags
    // before we create them.
    let mut program_offset = args.len() as usize
        + (Init::len() + args.header_len()) * programs.len()
        + (XousKernel::len() + args.header_len());
    let xkrn = XousKernel::new(
        program_offset as u32,
        kernel.program.len() as u32,
        kernel.text_offset,
        kernel.data_offset,
        kernel.data_size,
        kernel.entry_point,
    );
    program_offset += kernel.program.len();
    args.add(xkrn);

    for program_description in &programs {
        let init = Init::new(
            program_offset as u32,
            program_description.program.len() as u32,
            program_description.text_offset,
            program_description.data_offset,
            program_description.data_size,
            program_description.entry_point,
        );
        program_offset += program_description.program.len();
        args.add(init);
    }

    if matches.is_present("debug") {
        args.add(Bflg::new().debug());
    }

    println!("Arguments: {}", args);

    let output_filename = matches
        .value_of("output")
        .expect("output filename not present");
    let mut f = File::create(output_filename)
        .expect(&format!("Couldn't create output file {}", output_filename));
    args.write(&f).expect("Couldn't write to args");

    pad_file_to_4_bytes(&mut f);
    f.write(&kernel.program).expect("Couldn't write kernel");

    for program_description in &programs {
        pad_file_to_4_bytes(&mut f);
        f.write(&program_description.program)
            .expect("Couldn't write kernel");
    }

    println!(
        "Runtime will require {} bytes to track memory allocations",
        memory_required
    );
    println!("Image created in file {}", output_filename);
}
