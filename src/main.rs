use std::{fs::File, io::Read};

#[derive(Debug)]
#[allow(dead_code)]
struct GzipMeta {
    id1: u8,
    id2: u8,
    cm: u8,
    flg: u8,
    mtime: u32,
    xfl: u8,
    os: u8,
    filename: Option<String>,
}

#[allow(dead_code)]
struct Gzip {
    meta: GzipMeta,
    data: Vec<u8>,
}

pub mod flag {
    pub const FTEXT: u8 = 1 << 0;
    pub const FHCRC: u8 = 1 << 1;
    pub const FEXTRA: u8 = 1 << 2;
    pub const FNAME: u8 = 1 << 3;
    pub const FCOMMENT: u8 = 1 << 4;
}

#[allow(dead_code)]
fn deflate() -> Vec<u8> {
    Vec::new()
}

#[allow(dead_code)]
fn gzip() -> Vec<u8> {
    Vec::new()
}

#[allow(dead_code)]
#[allow(unused)]
fn inflate(bytes: &[u8]) -> Result<Vec<u8>, &str> {
    Ok(Vec::new())
}

fn parse_gzip_meta(bytes: &[u8]) -> Result<GzipMeta, &str> {
    if bytes.len() < 10 {
        return Err("size should greater than 10");
    }

    for byte in bytes {
        print!("0x{:02x} ", byte);
    }
    println!("");

    if bytes[0] != 0x1f || bytes[1] != 0x8b || bytes[2] != 0x08 {
        return Err("magic number");
    }

    let id1 = bytes[0];
    let id2 = bytes[1];
    let cm = bytes[2];
    let flg = bytes[3];

    let mtime = (bytes[4] as u32)
        + ((bytes[5] as u32) << 8)
        + ((bytes[6] as u32) << 16)
        + ((bytes[7] as u32) << 24);

    let xfl = bytes[8];
    let os = bytes[9];

    let mut index = 10;
    if (flg & flag::FEXTRA) != 0 {}

    let mut filename = None;
    if (flg & flag::FNAME) != 0 {
        let start = index;
        while bytes[index] != 0 {
            index += 1;
        }
        let s = &bytes[start..index];
        filename = Some(String::from_utf8_lossy(s).to_string());
    }

    if (flg & flag::FCOMMENT) != 0 {}

    Ok(GzipMeta {
        id1,
        id2,
        cm,
        flg,
        mtime,
        xfl,
        os,
        filename,
    })
}

fn ungzip(bytes: &[u8]) -> Result<Gzip, &str> {
    let meta = parse_gzip_meta(bytes);
    if let Err(e) = meta {
        return Err(e);
    }

    println!("meta: {:?}", meta);

    let data = inflate(bytes);
    if let Err(e) = data {
        return Err(e);
    }

    Ok(Gzip {
        meta: meta.unwrap(),
        data: data.unwrap(),
    })
}

fn main() {
    let file = File::open("test_files/a.gz");
    let mut buf: Vec<u8> = Vec::new();
    let _ = file.unwrap().read_to_end(&mut buf);
    let result = ungzip(&buf).unwrap();
    println!("{:?}", result.data);
}
