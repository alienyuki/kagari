use std::{fs::{self, File}, io::Read, str};

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

fn get_len(bs: &mut BitStream, len_code: u16) -> u16 {
    let code_table = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];

    let extra_table = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];

    // The extra bits should be interpreted as a machine integer
    // stored with the most-significant bit first
    println!("{len_code}");
    code_table[len_code as usize] + bs.get_nbit(extra_table[len_code as usize])
}

fn get_dis(bs: &mut BitStream, dis_code: u16) -> u16 {
    let code_table = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];

    let extra_table = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13
    ];

    code_table[dis_code as usize] + bs.get_nbit(extra_table[dis_code as usize])
}

fn inflate_fixed_huff(bs: &mut BitStream, v: &mut Vec<u8>) {
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
        let mut huff_key = bs.get_nbit_rev(7);
        println!("a: {huff_key}");
        if huff_key <= 0b0010111 {
            // 256-279
            if huff_key == 0 {
                break;
            } else {
                let len_code = huff_key - 1;
                let len = get_len(bs, len_code);

                // Distance codes 0-31 are represented by (fixed-length) 5-bit codes
                let dis_code = bs.get_nbit_rev(5);
                let dis = get_dis(bs, dis_code);

                // copy string
                // Note also that the referenced string may overlap the current position
                let index = v.len() - dis as usize;
                for i in index..(index + len as usize) {
                    v.push(v[i]);
                }
            }

            continue;
        }
        let c = bs.get_nbit(1);
        huff_key = (huff_key << 1) + c;
        if huff_key <= 0b10111111 {
            // 0-143
            let p = (huff_key - 0b00110000 as u16) as u8;
            v.push(p);
            continue;

        } else if huff_key <= 0b11000111 {
            // 280-287, the same as 256-279
            let len_code = huff_key - 0b11000000 + 279 - 256;
            println!("len code: {}, a: {}\n", len_code, huff_key);
            let len = get_len(bs, len_code);

            // Distance codes 0-31 are represented by (fixed-length) 5-bit codes
            let dis_code = bs.get_nbit_rev(5);
            let dis = get_dis(bs, dis_code);

            // copy string
            // Note also that the referenced string may overlap the current position
            let index = v.len() - dis as usize;
            for i in index..(index + len as usize) {
                v.push(v[i]);
            }
            continue;
        }

        // 144-255
        let c = bs.get_nbit(1);
        huff_key = (huff_key << 1) + c;
        let p = (huff_key - 0b110010000 as u16) as u8;
        v.push(p);
    }
}

#[allow(dead_code)]
fn inflate(bytes: &[u8]) -> Result<Vec<u8>, &str> {
    let mut v = Vec::new();
    let mut bs = BitStream::new(bytes);

    let bfinal = bs.get_nbit(1);
    let btype = bs.get_nbit(2);

    if bfinal != 1 {
        todo!("not final block");
    }

    if btype as u8 == FIXED_HUFF {
        inflate_fixed_huff(&mut bs, &mut v);
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

fn generate_cases() -> Vec<String> {
    let path = "./test_files";
    let mut v = Vec::new();

    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let filename = entry.file_name().into_string().unwrap();
        if !filename.ends_with(".gz") {
            v.push(filename);
        }
    }
    v
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("{:?}", args);

    let mut res = Vec::new();

    let mut cases = &args[1..args.len()];
    let default_cases = generate_cases();

    if cases.len() == 0 {
        cases = &default_cases[0..default_cases.len()];
    }

    for arg in cases {
        println!("{arg}");

        let gz_path = format!("test_files/{}.gz", arg);
        let mut gz_content: Vec<u8> = Vec::new();
        let _ = File::open(&gz_path).unwrap().read_to_end(&mut gz_content);

        let result = ungzip(&gz_content).unwrap();
        println!("{:?}", result.data);
        // let s = str::from_utf8(&result.data).unwrap();

        let raw_path = format!("test_files/{}", arg);
        let mut file_content: Vec<u8> = Vec::new();
        let _ = File::open(&raw_path).unwrap().read_to_end(&mut file_content);

        println!("aa: {:?}, {:?}", file_content, result.data);

        if file_content == result.data {
            println!("\x1b[32m{arg} pass.\x1b[0m");
        } else {
            println!("\x1b[1;31m{arg} error!\x1b[0m");
            res.push(arg);
        }
    }

    if res.len() != 0 {
        println!("\x1b[1;31m{:?} is not passed!\x1b[0m", res);
    } else {
        println!("\x1b[32mAll passed!\x1b[0m");
    }
}
