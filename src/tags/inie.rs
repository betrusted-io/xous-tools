use crate::elf::MiniElfSection;
use crate::xous_arguments::{XousArgument, XousArgumentCode, XousSize};
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct IniE {
    /// Address of Init in RAM (i.e. SPI flash)
    load_offset: u32,

    /// Virtual address entry point
    entrypoint: u32,

    /// Array of minielf sections
    sections: Vec<MiniElfSection>,

    /// Actual program data
    data: Vec<u8>,
}

impl fmt::Display for IniE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "    IniE: entrypoint @ {:08x}, loaded from {:08x}.  Sections:",
            self.entrypoint, self.load_offset
        )?;
        let mut load_offset = self.load_offset;
        for section in &self.sections {
            writeln!(f, "        Loaded from {:08x} - {}", load_offset, section)?;
            load_offset += section.size as u32;
        }
        Ok(())
    }
}

impl IniE {
    pub fn new(entrypoint: u32, sections: Vec<MiniElfSection>, data: Vec<u8>) -> IniE {
        IniE {
            load_offset: 0,
            entrypoint,
            sections,
            data,
        }
    }
}

impl XousArgument for IniE {
    fn code(&self) -> XousArgumentCode {
        make_type!("IniE")
    }

    fn length(&self) -> XousSize {
        4 + 4 + (self.sections.len() * 8) as XousSize
    }

    fn finalize(&mut self, offset: usize) -> usize {
        self.load_offset = offset as u32;
        self.data.len()
    }

    fn last_data(&self) -> &[u8] {
        &self.data
    }

    fn serialize(&self, output: &mut dyn io::Write) -> io::Result<usize> {
        let mut written = 0;
        written += output.write(&self.load_offset.to_le_bytes())?;
        written += output.write(&self.entrypoint.to_le_bytes())?;
        for section in &self.sections {
            written += output.write(&section.virt.to_le_bytes())?;
            let mut word2 = (section.size >> 2).to_le_bytes();
            word2[3] = section.flags.bits();
            written += output.write(&word2)?;
        }
        Ok(written)
    }
}
