use log::debug;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use xmas_elf::program::Type as ProgramType;
use xmas_elf::sections::ShType;
use xmas_elf::ElfFile;

pub struct ProgramDescription {
    /// Virtual address of .text section in RAM
    pub text_offset: u32,

    /// Virtual address of .data and .bss section in RAM
    pub data_offset: u32,

    /// Size of .data and .bss section
    pub data_size: u32,

    /// Virtual address of the entrypoint
    pub entry_point: u32,

    /// Program contents
    pub program: Vec<u8>,
}

struct ElfHeader {
    virt: usize,
    phys: usize,
    length: usize,
}

#[derive(Debug)]
pub enum ElfReadError {
    /// Read an unexpected number of bytes
    WrongReadSize(u64 /* expected */, u64 /* actual */),

    /// "Couldn't seek to end of file"
    SeekFromEndError(std::io::Error),

    /// Couldn't read ELF file
    ReadFileError(std::io::Error),

    /// Couldn't open the ELF file
    OpenElfError(std::io::Error),

    /// Couldn't parse the ELF file
    ParseElfError(&'static str),

    /// Section wasn't in range
    SectionRangeError,

    /// Couldn't seek the file to write the section
    FileSeekError(std::io::Error),

    /// Couldn't write the section to the file
    WriteSectionError(std::io::Error),
}

pub fn read_program(filename: &str) -> Result<ProgramDescription, ElfReadError> {
    let mut headers = vec![];

    let mut b = Vec::new();
    {
        let mut fi = File::open(filename).or_else(|x| Err(ElfReadError::OpenElfError(x)))?;
        fi.read_to_end(&mut b)
            .or_else(|x| Err(ElfReadError::ReadFileError(x)))?;
    }
    let elf = ElfFile::new(&b).or_else(|x| Err(ElfReadError::ParseElfError(x)))?;
    let entry_point = elf.header.pt2.entry_point() as u32;
    let mut program_data = Cursor::new(Vec::new());

    let mut expected_size = 0;
    let mut size = 0;
    let mut data_offset = 0;
    let mut data_size = 0;
    let mut text_offset = 0;
    let mut phys_offset = 0;

    debug!("ELF: {:?}", elf.header);
    for ph in elf.program_iter() {
        debug!("Program Header: {:?}", ph);
        if ph.get_type() == Ok(ProgramType::Load) {
            expected_size += ph.file_size();
            headers.push(ElfHeader {
                virt: ph.virtual_addr() as usize,
                phys: ph.physical_addr() as usize,
                length: ph.file_size() as usize,
            });
            if phys_offset == 0 {
                phys_offset = ph.physical_addr();
            }
        }
        debug!("Physical address: {:08x}", ph.physical_addr());
        debug!("Virtual address: {:08x}", ph.virtual_addr());
        debug!("Offset: {:08x}", ph.offset());
        debug!("Size: {:08x}", ph.file_size());
    }
    debug!(
        "File should be {} bytes, and program starts at 0x{:x}",
        expected_size, entry_point
    );

    for s in elf.section_iter() {
        let name = s.get_name(&elf).unwrap_or("<<error>>");
        // if s.get_type() == Ok(ShType::NoBits) {
        //     debug!("(Skipping section {} -- invalid type)", name);
        //     continue;
        // }

        if s.address() == 0 {
            debug!("(Skipping section {} -- invalid address)", name);
            continue;
        }

        debug!("Section {}:", name);
        debug!("    flags:            {:?}", s.flags());
        debug!("    type:             {:?}", s.get_type());
        debug!("    address:          {:08x}", s.address());
        debug!("    offset:           {:08x}", s.offset());
        debug!("    size:             {:?}", s.size());
        debug!("    link:             {:?}", s.link());
        size += s.size();
        if size & 3 != 0 {
            debug!("Size is not padded!");
        }

        if name == ".data" {
            data_offset = s.address() as u32;
            data_size += s.size() as u32;
        } else if s.get_type() == Ok(ShType::NoBits) {
            // Add bss-type sections to the data section
            data_size += s.size() as u32;
            debug!("Skipping {} because nobits", name);
            continue;
        } else if text_offset == 0 && s.size() != 0 {
            text_offset = s.address() as u32;
        }
        if s.size() == 0 {
            debug!("Skipping {} because size is 0", name);
            continue;
        }
        debug!("Adding {} to the file", name);
        let header = headers
            .iter()
            .find(|&x| {
                debug!(
                    "Comparing {:08x}:{:08x} to {:08x}:{:08x}",
                    s.address(),
                    s.address() + s.size(),
                    x.virt,
                    x.virt + x.length
                );
                (s.address() as usize) >= x.virt
                    && ((s.address() + s.size()) as usize) <= x.virt + x.length
            })
            .ok_or(ElfReadError::SectionRangeError)?;
        let program_offset = s.address() - header.virt as u64 + header.phys as u64 - phys_offset;
        debug!(
            "s offset: {:08x}  program_offset: {:08x}  Bytes: {}  seek: {}",
            s.offset(),
            program_offset,
            s.raw_data(&elf).len(),
            program_offset
        );
        program_data
            .seek(SeekFrom::Start(program_offset))
            .or_else(|x| Err(ElfReadError::FileSeekError(x)))?;
        let section_data = s.raw_data(&elf);
        program_data
            .write(section_data)
            .or_else(|x| Err(ElfReadError::WriteSectionError(x)))?;
    }
    let observed_size = program_data
        .seek(SeekFrom::End(0))
        .or_else(|e| Err(ElfReadError::SeekFromEndError(e)))?;
    if expected_size != observed_size {
        Err(ElfReadError::WrongReadSize(expected_size, observed_size))
    } else {
        Ok(ProgramDescription {
            entry_point,
            program: program_data.into_inner(),
            data_size,
            data_offset,
            text_offset,
        })
    }
}
