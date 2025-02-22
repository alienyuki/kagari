use std::{
    collections::HashMap,
    fs::{self, File},
    hash::Hash,
    hash::Hasher,
    io::Read,
    panic, str,
};

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
    crc32: u32,
    isize: u32,
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
    code_table[len_code as usize] + bs.get_nbit(extra_table[len_code as usize])
}

fn get_dis(bs: &mut BitStream, dis_code: u16) -> u16 {
    let code_table = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];

    let extra_table = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];

    code_table[dis_code as usize] + bs.get_nbit(extra_table[dis_code as usize])
}

#[derive(Debug)]
struct HuffItem {
    len: u8,
    key: u16,
}

struct HuffCode {
    hm: HashMap<HuffItem, u16>,
}

impl Hash for HuffItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len.hash(state);
        self.key.hash(state);
    }
}

impl PartialEq for HuffItem {
    fn eq(&self, rhs: &HuffItem) -> bool {
        if rhs.len == self.len && rhs.key == self.key {
            return true;
        }
        false
    }
}

impl Eq for HuffItem {}

impl HuffCode {
    fn find(&self, key: u16, len: u8) -> Option<u16> {
        let huff_k = HuffItem { len, key };
        if let Some(code) = self.hm.get(&huff_k) {
            return Some(*code);
        }
        None
    }
}

fn build_huff(bit_length: &[u8]) -> HuffCode {
    let mut ret = HuffCode { hm: HashMap::new() };
    let mut code = 0;
    let mut bl_count = Vec::new();
    for _ in 0..bit_length.len() {
        bl_count.push(0);
    }

    for i in bit_length {
        if *i != 0 {
            bl_count[*i as usize] += 1;
        }
    }

    let mut next_code = [0; 300];
    for bits in 1..bl_count.len() {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }

    for i in 0..bit_length.len() {
        if bit_length[i] != 0 {
            let hi = HuffItem {
                len: bit_length[i],
                key: next_code[bit_length[i] as usize],
            };
            ret.hm.insert(hi, i as u16);
            next_code[bit_length[i] as usize] += 1;
        }
    }
    ret
}

fn valid_huff(huff: &[u8]) {
    let mut check = 0.0;
    for i in huff {
        if *i != 0 {
            check += 1.0 / ((1 << i) as f64);
        }
    }

    if check != 1.0 {
        panic!("Not a valid huff!");
    }
}

fn generate_hf_from_cl(bs: &mut BitStream, cl_hf: &HuffCode, size: u16) -> HuffCode {
    let mut hf_vec = Vec::new();
    while hf_vec.len() != size as usize {
        let value = get_code_from_huff(bs, cl_hf);
        // For even greater compactness, the code length sequences themselves
        // are compressed using a Huffman code. The alphabet for code lengths
        // is as follows:
        if value <= 15 {
            hf_vec.push(value as u8);
        } else if value == 16 {
            let last_value = hf_vec[hf_vec.len() - 1];
            for _ in 0..(bs.get_nbit(2) + 3) {
                hf_vec.push(last_value);
            }
        } else if value == 17 {
            for _ in 0..(bs.get_nbit(3) + 3) {
                hf_vec.push(0);
            }
        } else if value == 18 {
            for _ in 0..(bs.get_nbit(7) + 11) {
                hf_vec.push(0);
            }
        }
    }

    valid_huff(&hf_vec);
    build_huff(&hf_vec[..])
}

fn get_code_from_huff(bs: &mut BitStream, hf: &HuffCode) -> u16 {
    loop {
        let mut key = bs.get_bit();
        let mut len = 1;
        loop {
            if let Some(value) = hf.find(key, len) {
                return value;
            }
            key <<= 1;
            key += bs.get_bit();
            len += 1;
        }
    }
}

fn decode_dynamic_huff(bs: &mut BitStream, ll_hf: &HuffCode, dis_hf: &HuffCode) -> Vec<u8> {
    let mut ret = Vec::new();
    loop {
        let value = get_code_from_huff(bs, ll_hf);
        if value < 256 {
            ret.push(value as u8);
        } else if value == 256 {
            return ret;
        } else {
            let len_code = value - 257;
            let len = get_len(bs, len_code);

            let dis_code = get_code_from_huff(bs, dis_hf);
            let dis = get_dis(bs, dis_code);

            let index = ret.len() - dis as usize;
            for i in index..(index + len as usize) {
                ret.push(ret[i]);
            }
        }
    }
}

fn inflate_dynamic_huff(bs: &mut BitStream) -> Vec<u8> {
    let hlit = bs.get_nbit(5);
    let hdist = bs.get_nbit(5);
    let hclen = bs.get_nbit(4);

    let map = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut code_length = [0; 19];
    for i in 0..(hclen + 4) {
        code_length[map[i as usize]] = bs.get_nbit(3) as u8;
    }

    valid_huff(&code_length);

    // code length alphabet
    let cl_hf = build_huff(&code_length[..]);
    // generate huff literal/length tree
    let ll_hf = generate_hf_from_cl(bs, &cl_hf, hlit + 257);
    // generate huff distance tree
    let dis_hf = generate_hf_from_cl(bs, &cl_hf, hdist + 1);

    decode_dynamic_huff(bs, &ll_hf, &dis_hf)
}

fn inflate_fixed_huff(bs: &mut BitStream) -> Vec<u8> {
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
    let mut v = Vec::new();
    loop {
        let mut huff_key = bs.get_nbit_rev(7);
        // println!("huff_key: {huff_key}");
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
    v
}

#[allow(dead_code)]
fn inflate(bytes: &[u8]) -> Result<Vec<u8>, &str> {
    let mut bs = BitStream::new(bytes);
    let mut ret = Vec::new();

    loop {
        let bfinal = bs.get_nbit(1);
        let btype = bs.get_nbit(2);

        if btype as u8 == flag::FIXED_HUFF {
            ret.append(&mut inflate_fixed_huff(&mut bs));
        } else if btype as u8 == flag::DYNAMIC_HUFF {
            ret.append(&mut inflate_dynamic_huff(&mut bs));
        } else if btype as u8 == flag::NO_COMPRESSION {
            bs.clear_bits();
            let len = (bs.get_nbit(8)) + (bs.get_nbit(8) << 8);
            let nlen = (bs.get_nbit(8)) + (bs.get_nbit(8) << 8);

            if len + nlen != 65535 {
                println!("{len} {nlen}");
                panic!("nlen should be one's complement of len.");
            }
            let mut v = bs.get_nbytes(len as usize);
            ret.append(&mut v);
        }

        if bfinal == 1 {
            return Ok(ret);
        }
    }
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

    fn clear_bits(&mut self) {
        while self.mask != 1 {
            self.get_bit();
        }
    }

    fn get_nbytes(&mut self, len: usize) -> Vec<u8> {
        let mut v = Vec::new();
        for i in 0..len {
            v.push(self.bytes[i + self.index]);
        }
        self.index += len;
        v
    }
}

fn parse_gzip_meta(bytes: &[u8]) -> Result<GzipMeta, &str> {
    if bytes.len() < 10 {
        return Err("size should greater than 10");
    }

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

    let crc32_start = bytes.len() - 8;
    let crc32 = (bytes[crc32_start] as u32)
        + ((bytes[crc32_start + 1] as u32) << 8)
        + ((bytes[crc32_start + 2] as u32) << 16)
        + ((bytes[crc32_start + 3] as u32) << 24);

    let isize = (bytes[crc32_start + 4] as u32)
        + ((bytes[crc32_start + 5] as u32) << 8)
        + ((bytes[crc32_start + 6] as u32) << 16)
        + ((bytes[crc32_start + 7] as u32) << 24);

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
        crc32,
        isize,
    })
}

fn crc32(bytes: &[u8]) -> u32 {
    if bytes.len() == 0 {
        return 0;
    }

    let mut v: Vec<u8> = Vec::new();
    for byte in bytes {
        for i in 0..8 {
            if (*byte & (1 << i)) != 0 {
                v.push(1);
            } else {
                v.push(0);
            }
        }
    }

    for _ in 0..32 {
        v.push(0);
    }

    for i in 0..32 {
        v[i] ^= 1;
    }

    let divisor = vec![
        1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0,
        1, 1, 1,
    ];

    let mut vloop: Vec<u8> = Vec::new();
    for i in 0..divisor.len() {
        vloop.push(v[i]);
    }

    for i in 32..v.len() {
        vloop[32] = v[i];

        if vloop[0] != 0 {
            for j in 0..divisor.len() {
                vloop[j] ^= divisor[j];
            }
        }

        for j in 0..32 {
            vloop[j] = vloop[j + 1];
        }
    }

    let mut ret = 0;
    for i in 0..32 {
        ret <<= 1;
        ret += (vloop[31 - i] as u32) ^ 1;
    }
    ret
}

fn ungzip(bytes: &[u8]) -> Result<Gzip, &str> {
    let res_meta = parse_gzip_meta(bytes);
    if let Err(e) = res_meta {
        return Err(e);
    }

    let meta = res_meta.unwrap();
    let res_data = inflate(meta.cdata);

    if let Err(e) = res_data {
        return Err(e);
    }

    let data = res_data.unwrap();
    if data.len() != meta.isize as usize || crc32(&data) != meta.crc32 {
        return Err("check error");
    }

    Ok(Gzip { meta, data })
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
        let gz_path = format!("test_files/{}.gz", arg);
        let mut gz_content: Vec<u8> = Vec::new();
        let _ = File::open(&gz_path).unwrap().read_to_end(&mut gz_content);

        let result = ungzip(&gz_content).unwrap();
        // let s = str::from_utf8(&result.data).unwrap();

        let raw_path = format!("test_files/{}", arg);
        let mut file_content: Vec<u8> = Vec::new();
        let _ = File::open(&raw_path)
            .unwrap()
            .read_to_end(&mut file_content);

        // println!("aa: {:?}, {:?}", file_content, result.data);

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
