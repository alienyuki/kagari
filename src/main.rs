use std::{fs::File, io::Read};

use flag::FIXED_HUFF;

#[derive(Debug)]
#[allow(dead_code)]
struct GzipMeta<'a> {
    id1: u8,
    id2: u8,
    cm: u8,
    flg: u8,
    mtime: u32,
    xfl: u8,
    os: u8,
    filename: Option<String>,
    cdata: &'a [u8],
}

#[allow(dead_code)]
struct Gzip<'a> {
    meta: GzipMeta<'a>,
    data: Vec<u8>,
}

pub mod flag {
    pub const FTEXT: u8 = 1 << 0;
    pub const FHCRC: u8 = 1 << 1;
    pub const FEXTRA: u8 = 1 << 2;
    pub const FNAME: u8 = 1 << 3;
    pub const FCOMMENT: u8 = 1 << 4;

    pub const NO_COMPRESSION: u8 = 0;
    pub const FIXED_HUFF: u8 = 1;
    pub const DYNAMIC_HUFF: u8 = 2;
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
fn inflate(bytes: &[u8]) -> Result<Vec<u8>, &str> {
    let mut v = Vec::new();
    let mut bs = BitStream::new(bytes);

    let bfinal = bs.get_nbit(1);
    let btype = bs.get_nbit(2);

    
    if btype as u8 == FIXED_HUFF {
        /*
            Lit Value   Bits    Codes
            ---------   ----    -----
              0 - 143     8     00110000 through
                                10111111
            144 - 255     9     110010000 through
                                111111111
            256 - 279     7     0000000 through
                                0010111
            280 - 287     8     11000000 through
                                11000111
        */
        loop {
            let mut a = bs.get_nbit_rev(7);
            if a < 0b0010111 {
                // repeat
                if a == 0 {
                    break;
                }
            } else {
                let c = bs.get_nbit(1);
                a = (a << 1) + c;
                if a < 0b10111111 {
                    let p = (a - 0b00110000 as u16) as u8;
                    v.push(p);
                }
            }
        }
    }

    Ok(v)
}

struct BitStream<'a> {
    bytes: &'a [u8],
    mask: u8,
    index: usize,
}

impl<'a> BitStream<'a> {
    fn new(bytes: &[u8]) -> BitStream {
        BitStream {
            bytes,
            mask: 1,
            index: 0,
        }
    }

    fn get_bit(&mut self) -> u16 {
        let mut ret = 0;
        if (self.mask & self.bytes[self.index]) != 0 {
            ret = 1;
        }

        if self.mask << 1 == 0 {
            self.index += 1;
            self.mask = 1;
        } else {
            self.mask <<= 1;
        }

        ret
    }

    fn get_nbit(&mut self, cnt: u16) -> u16 {
        let mut n = 0;

        for i in 0..cnt {
            n |= self.get_bit() << i;
        }

        n
    }

    fn get_nbit_rev(&mut self, cnt: u16) -> u16 {
        let mut n = 0;

        for _ in 0..cnt {
            n <<= 1;
            let c = self.get_bit();
            n |= c;
        }

        n
    }
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
        index += 1;
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
        cdata: &bytes[index..(bytes.len() - 8)],
    })
}

fn ungzip(bytes: &[u8]) -> Result<Gzip, &str> {
    let res_meta = parse_gzip_meta(bytes);
    if let Err(e) = res_meta {
        return Err(e);
    }

    let meta = res_meta.unwrap();
    println!("meta: {:?}", meta);

    let data = inflate(meta.cdata);

    println!("{:?}", data);

    if let Err(e) = data {
        return Err(e);
    }

    Ok(Gzip {
        meta,
        data: data.unwrap(),
    })
}

fn main() {
    let file = File::open("test_files/y.gz");
    let mut buf: Vec<u8> = Vec::new();
    let _ = file.unwrap().read_to_end(&mut buf);
    let result = ungzip(&buf).unwrap();
    println!("{:?}", result.data);
}
