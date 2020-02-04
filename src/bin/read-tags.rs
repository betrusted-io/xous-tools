use std::fs::File;
use std::io;
use std::io::prelude::*;

unsafe fn process_tags(b8: *mut u8) {
    use core::mem::transmute;
    let b32 = transmute::<*mut u8, *mut u32>(b8);
    let b16 = transmute::<*mut u8, *mut u16>(b8);
    let mut byte_offset = 0;
    let mut total_words = 0u32;
    loop {
        let tag_name = u32::from_le(b32.add(byte_offset / 4).read()) as u32;
        byte_offset += 4;
        let crc = u16::from_le(b16.add(byte_offset / 2).read()) as u32;
        byte_offset += 2;
        let size = u16::from_le(b16.add(byte_offset / 2).read()) as u32 * 4;
        byte_offset += 2;

        print!("{:08x} ({} bytes, crc: {}):", tag_name, size, crc);
        let mut offset = 0;
        while offset < size {
            if total_words == 0 {
                total_words = b32.add(byte_offset / 4).read() * 4;
            }
            print!(" {:08x}", b32.add(byte_offset / 4).read());
            offset = offset + 4;
            byte_offset = byte_offset + 4;
        }
        println!("");

        if byte_offset as u32 >= total_words {
            return;
        }
    }
}

fn doit() -> io::Result<()> {
    let mut tag_buf = vec![];
    {
        let mut f = File::open("args.bin")?;
        f.read_to_end(&mut tag_buf)?;
    }

    let byte_buffer = tag_buf.as_mut_ptr();
    unsafe { process_tags(byte_buffer) };
    Ok(())
}
fn main() {
    doit().unwrap();
}
