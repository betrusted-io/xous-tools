use crate::xous_arguments::{XousArgument, XousArgumentCode, XousSize};
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct MemoryRegion {
    /// Starting offset (in bytes)
    start: u32,

    /// Length (in bytes)
    length: u32,

    /// Region name (as a type)
    name: XousArgumentCode,
}

pub struct MemoryRegions {
    regions: Vec<MemoryRegion>,
}

impl fmt::Display for MemoryRegions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Additional regions:")?;
        for region in &self.regions {
            let tag_name_bytes = region.name.to_le_bytes();
            let tag_name_str = String::from_utf8_lossy(&tag_name_bytes);
            writeln!(
                f,
                "    {} ({:08x}): {:08x} - {:08x}",
                tag_name_str, region.name,
                region.start,
                region.start + region.length
            )?;
        }
        Ok(())
    }
}

impl MemoryRegion {
    pub fn new(start: XousSize, length: XousSize, name: &str) -> MemoryRegion {
        MemoryRegion { start, length, name: make_type!(name) }
    }
}

impl MemoryRegions {
    pub fn new() -> MemoryRegions {
        MemoryRegions { regions: vec![] }
    }
    pub fn add(&mut self, region: MemoryRegion) {
        self.regions.push(region)
    }
    pub fn len(&self) -> usize {
        self.regions.len()
    }
}

impl XousArgument for MemoryRegions {
    fn code(&self) -> XousArgumentCode {
        make_type!("MBLK")
    }
    fn length(&self) -> XousSize {
        (self.regions.len() * std::mem::size_of::<MemoryRegion>()) as XousSize
    }
    fn serialize(&self, output: &mut dyn io::Write) -> io::Result<usize> {
        let mut written = 0;
        for region in &self.regions {
            written = written + output.write(&region.start.to_le_bytes())?;
            written = written + output.write(&region.length.to_le_bytes())?;
            written = written + output.write(&region.name.to_le_bytes())?;
        }
        Ok(written)
    }
}
