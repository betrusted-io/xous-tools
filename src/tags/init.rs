use crate::xous_arguments::{XousArgument, XousArgumentCode, XousSize};
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct Init {
    /// Address of Init in RAM (i.e. SPI flash)
    load_offset: u32,

    /// Virtual address of .text section in RAM
    text_offset: u32,

    /// Size of the text section
    text_size: u32,

    /// Virtual address of .data section in RAM
    data_offset: u32,

    /// Size of .data section
    data_size: u32,

    /// Size of the .bss section
    bss_size: u32,

    /// Virtual address entry point
    entrypoint: u32,
}

impl fmt::Display for Init {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "    init: {} bytes long, loaded from {:08x} to {:08x} with entrypoint @ {:08x} and {} bytes of data @ {:08x}, {} bytes of .bss",
            self.text_size, self.load_offset, self.text_offset, self.entrypoint,
            self.data_size, self.data_offset, self.bss_size)
    }
}

impl Init {
    pub fn new(
        load_offset: u32,
        text_offset: u32,
        text_size: u32,
        data_offset: u32,
        data_size: u32,
        bss_size: u32,
        entrypoint: u32,
    ) -> Init {
        Init {
            load_offset,
            text_offset,
            text_size,
            data_offset,
            data_size,
            bss_size,
            entrypoint,
        }
    }
    pub fn len() -> usize {
        std::mem::size_of::<Self>()
    }
}

impl XousArgument for Init {
    fn code(&self) -> XousArgumentCode {
        make_type!("Init")
    }
    fn length(&self) -> XousSize {
        std::mem::size_of::<Self>() as XousSize
    }
    fn serialize(&self, output: &mut dyn io::Write) -> io::Result<usize> {
        let mut written = 0;
        written += output.write(&self.load_offset.to_le_bytes())?;
        written += output.write(&self.text_offset.to_le_bytes())?;
        written += output.write(&self.text_size.to_le_bytes())?;
        written += output.write(&self.data_offset.to_le_bytes())?;
        written += output.write(&self.data_size.to_le_bytes())?;
        written += output.write(&self.bss_size.to_le_bytes())?;
        written += output.write(&self.entrypoint.to_le_bytes())?;
        Ok(written)
    }
}
