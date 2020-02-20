extern crate xous_tools;
extern crate xmas_elf;
#[macro_use]
extern crate clap;

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use xous_tools::tags::init::Init;
use xous_tools::tags::memory::{MemoryRegion, MemoryRegions};
use xous_tools::tags::xkrn::XousKernel;
use xous_tools::utils::{parse_csr_csv, parse_u32};
use xous_tools::xous_arguments::XousArguments;

use xmas_elf::program::Type as ProgramType;
use xmas_elf::sections::ShType;
use xmas_elf::ElfFile;

use clap::{App, Arg};

struct ProgramDescription {
    /// Virtual address of .text section in RAM
    text_offset: u32,

    /// Virtual address of .data and .bss section in RAM
    data_offset: u32,

    /// Size of .data and .bss section
    data_size: u32,

    /// Virtual address of the entrypoint
    entry_point: u32,

    /// Program contents
    program: Vec<u8>,
}

fn read_program(filename: &str) -> Result<ProgramDescription, &str> {
    let mut b = Vec::new();
    {
        let mut fi = File::open(filename).expect("Couldn't open program");
        fi.read_to_end(&mut b).expect("Couldn't read elf file");
    }
    let elf = ElfFile::new(&b).expect("Couldn't parse elf file");
    let entry_point = elf.header.pt2.entry_point() as u32;
    let mut program_data = Cursor::new(Vec::new());

    let mut expected_size = 0;
    let mut program_offset = 0;
    let mut size = 0;
    let mut data_offset = 0;
    let mut data_size = 0;
    let mut text_offset = 0;

    for ph in elf.program_iter() {
        println!("Program Header: {:?}", ph);
        if ph.get_type() == Ok(ProgramType::Load) && ph.flags().is_execute() {
            expected_size = ph.file_size();
            program_offset = ph.offset();
        }
        println!("Physical address: {:08x}", ph.physical_addr());
        println!("Virtual address: {:08x}", ph.virtual_addr());
        println!("Offset: {:08x}", ph.offset());
        println!("Size: {:08x}", ph.file_size());
    }
    println!(
        "File should be {} bytes, and program starts at 0x{:x}",
        expected_size, program_offset
    );

    for s in elf.section_iter() {
        let name = s.get_name(&elf).unwrap_or("<<error>>");
        // if s.get_type() == Ok(ShType::NoBits) {
        //     println!("(Skipping section {} -- invalid type)", name);
        //     continue;
        // }

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

        if name == ".data" {
            data_offset = s.address() as u32;
            data_size += s.size() as u32;
        } else if s.get_type() == Ok(ShType::NoBits) {
            data_size += s.size() as u32;
            continue;
        } else if text_offset == 0 && s.size() != 0 {
            text_offset = s.address() as u32;
        }
        if s.size() == 0 {
            continue;
        }
        program_data
            .seek(SeekFrom::Start(s.offset() - program_offset))
            .expect("Couldn't seek file");
        let section_data = s.raw_data(&elf);
        program_data
            .write(section_data)
            .expect("Couldn't save data");
    }
    let observed_size = program_data
        .seek(SeekFrom::End(0))
        .expect("Couldn't seek to end of file");
    if expected_size != observed_size {
        panic!(
            "Expected to read {} bytes, but actually read {} bytes",
            expected_size, observed_size
        );
    }

    Ok(ProgramDescription {
        entry_point,
        program: program_data.into_inner(),
        data_size,
        data_offset,
        text_offset,
    })
}

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
        let found_ram_name = MemoryRegion::make_name(&found_ram_name.unwrap());
        for (k, v) in &hv.regions {
            let region_name = MemoryRegion::make_name(k);
            // DOn't add the RAM section to the extra regions block.
            if region_name == found_ram_name {
                ram_name = region_name;
                continue;
            }
            regions.add(MemoryRegion::new(v.start, v.length, region_name));
            memory_required += ram_size / 4096;
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

    println!("Runtime will require {} bytes to track memory allocations", memory_required);
    println!("Image created in file {}", output_filename);
}
