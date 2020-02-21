extern crate xous_tools;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use xous_tools::make_type;

fn read_next_tag(b8: *mut u8, byte_offset: &mut usize) -> Result<(u32, u32, u32), ()> {
    let tag_name = u32::from_le(unsafe { (b8 as *mut u32).add(*byte_offset / 4).read() }) as u32;
    *byte_offset += 4;
    let crc = u16::from_le(unsafe { (b8 as *mut u16).add(*byte_offset / 2).read() }) as u32;
    *byte_offset += 2;
    let size = u16::from_le(unsafe { (b8 as *mut u16).add(*byte_offset / 2).read() }) as u32 * 4;
    *byte_offset += 2;
    Ok((tag_name, crc, size))
}

fn process_tag(b8: *mut u8, size: u32, byte_offset: &mut usize) -> Result<(), ()> {
    let mut offset = 0;

    while offset < size {
        print!(" {:08x}", unsafe {
            (b8 as *mut u32).add(*byte_offset / 4).read()
        });
        offset = offset + 4;
        *byte_offset = *byte_offset + 4;
    }
    println!("");
    Ok(())
}

fn process_tags(b8: *mut u8) {
    let mut byte_offset = 0;
    let mut total_words = 0u32;
    loop {
        let (tag_name, crc, size) =
            read_next_tag(b8, &mut byte_offset).expect("couldn't read next tag");
        if tag_name == make_type!("XArg") && size == 20 {
            total_words = unsafe { (b8 as *mut u32).add(byte_offset / 4).read() } * 4;
            println!(
                "Found Xous Args Size at offset {}, setting total_words to {}",
                byte_offset, total_words
            );
        }

        let tag_name_bytes = tag_name.to_le_bytes();
        let tag_name_str = String::from_utf8_lossy(&tag_name_bytes);
        print!(
            "{:08x} ({}) ({} bytes, crc: {}):",
            tag_name, tag_name_str, size, crc
        );
        process_tag(b8, size, &mut byte_offset).expect("couldn't read next data");

        if byte_offset as u32 == total_words {
            return;
        }
        if byte_offset as u32 > total_words {
            panic!(
                "exceeded total words ({}) with byte_offset of {}",
                total_words, byte_offset
            );
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
    process_tags(byte_buffer);
    Ok(())
}
fn main() {
    doit().unwrap();
}
