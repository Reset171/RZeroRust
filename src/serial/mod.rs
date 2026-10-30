use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

pub const FORMAT_VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RZTag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(u8, Vec<RZTag>),
    Compound(HashMap<String, RZTag>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl RZTag {
    pub fn id(&self) -> u8 {
        match self {
            Self::Byte(_) => 1,
            Self::Short(_) => 2,
            Self::Int(_) => 3,
            Self::Long(_) => 4,
            Self::Float(_) => 5,
            Self::Double(_) => 6,
            Self::ByteArray(_) => 7,
            Self::String(_) => 8,
            Self::List(_, _) => 9,
            Self::Compound(_) => 10,
            Self::IntArray(_) => 11,
            Self::LongArray(_) => 12,
        }
    }
}

pub struct RZNbt;

impl RZNbt {
    pub fn encode(root: &HashMap<String, RZTag>) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);
        Self::write_compound_body(&mut buf, root);
        buf
    }

    pub fn decode(data: &[u8]) -> HashMap<String, RZTag> {
        let mut pos = 0;
        Self::read_compound_body(data, &mut pos)
    }

    fn write_compound_body(buf: &mut Vec<u8>, map: &HashMap<String, RZTag>) {
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        for key in keys {
            if let Some(tag) = map.get(key) {
                buf.push(tag.id());
                Self::write_string(buf, key);
                Self::write_tag(buf, tag);
            }
        }
        buf.push(0);
    }

    fn write_tag(buf: &mut Vec<u8>, tag: &RZTag) {
        match tag {
            RZTag::Byte(v) => buf.push(*v as u8),
            RZTag::Short(v) => buf.extend_from_slice(&v.to_le_bytes()),
            RZTag::Int(v) => Self::write_var_int(buf, *v),
            RZTag::Long(v) => Self::write_var_long(buf, *v),
            RZTag::Float(v) => buf.extend_from_slice(&v.to_bits().to_le_bytes()),
            RZTag::Double(v) => buf.extend_from_slice(&v.to_bits().to_le_bytes()),
            RZTag::ByteArray(arr) => {
                Self::write_var_u32(buf, arr.len() as u32);
                buf.extend_from_slice(arr);
            }
            RZTag::String(s) => Self::write_string(buf, s),
            RZTag::List(elem_id, items) => {
                buf.push(*elem_id);
                Self::write_var_u32(buf, items.len() as u32);
                for item in items {
                    Self::write_tag(buf, item);
                }
            }
            RZTag::Compound(map) => Self::write_compound_body(buf, map),
            RZTag::IntArray(arr) => {
                Self::write_var_u32(buf, arr.len() as u32);
                for &v in arr {
                    buf.extend_from_slice(&v.to_le_bytes());
                }
            }
            RZTag::LongArray(arr) => {
                Self::write_var_u32(buf, arr.len() as u32);
                for &v in arr {
                    buf.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
    }

    fn write_var_u32(buf: &mut Vec<u8>, mut v: u32) {
        while (v & !0x7F) != 0 {
            buf.push(((v & 0x7F) as u8) | 0x80);
            v >>= 7;
        }
        buf.push(v as u8);
    }

    fn write_var_int(buf: &mut Vec<u8>, v: i32) {
        let z = ((v << 1) ^ (v >> 31)) as u32;
        Self::write_var_u32(buf, z);
    }

    fn write_var_long(buf: &mut Vec<u8>, v: i64) {
        let mut z = ((v << 1) ^ (v >> 63)) as u64;
        while (z & !0x7F) != 0 {
            buf.push(((z & 0x7F) as u8) | 0x80);
            z >>= 7;
        }
        buf.push(z as u8);
    }

    fn write_string(buf: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        Self::write_var_u32(buf, bytes.len() as u32);
        buf.extend_from_slice(bytes);
    }

    fn read_compound_body(buf: &[u8], pos: &mut usize) -> HashMap<String, RZTag> {
        let mut out = HashMap::new();
        while *pos < buf.len() {
            let id = buf[*pos];
            *pos += 1;
            if id == 0 {
                break;
            }
            let key = Self::read_string(buf, pos);
            let tag = Self::read_tag(buf, pos, id);
            out.insert(key, tag);
        }
        out
    }

    fn read_tag(buf: &[u8], pos: &mut usize, id: u8) -> RZTag {
        match id {
            1 => {
                let v = buf.get(*pos).copied().unwrap_or(0) as i8;
                *pos += 1;
                RZTag::Byte(v)
            }
            2 => {
                let mut b = [0u8; 2];
                b.copy_from_slice(&buf[*pos..*pos + 2]);
                *pos += 2;
                RZTag::Short(i16::from_le_bytes(b))
            }
            3 => RZTag::Int(Self::read_var_int(buf, pos)),
            4 => RZTag::Long(Self::read_var_long(buf, pos)),
            5 => {
                let mut b = [0u8; 4];
                b.copy_from_slice(&buf[*pos..*pos + 4]);
                *pos += 4;
                RZTag::Float(f32::from_bits(u32::from_le_bytes(b)))
            }
            6 => {
                let mut b = [0u8; 8];
                b.copy_from_slice(&buf[*pos..*pos + 8]);
                *pos += 8;
                RZTag::Double(f64::from_bits(u64::from_le_bytes(b)))
            }
            7 => {
                let n = Self::read_var_u32(buf, pos) as usize;
                let out = buf[*pos..*pos + n].to_vec();
                *pos += n;
                RZTag::ByteArray(out)
            }
            8 => RZTag::String(Self::read_string(buf, pos)),
            9 => {
                let elem_id = buf[*pos];
                *pos += 1;
                let n = Self::read_var_u32(buf, pos) as usize;
                let mut list = Vec::with_capacity(n);
                for _ in 0..n {
                    list.push(Self::read_tag(buf, pos, elem_id));
                }
                RZTag::List(elem_id, list)
            }
            10 => RZTag::Compound(Self::read_compound_body(buf, pos)),
            11 => {
                let n = Self::read_var_u32(buf, pos) as usize;
                let mut arr = Vec::with_capacity(n);
                for _ in 0..n {
                    let mut b = [0u8; 4];
                    b.copy_from_slice(&buf[*pos..*pos + 4]);
                    *pos += 4;
                    arr.push(i32::from_le_bytes(b));
                }
                RZTag::IntArray(arr)
            }
            12 => {
                let n = Self::read_var_u32(buf, pos) as usize;
                let mut arr = Vec::with_capacity(n);
                for _ in 0..n {
                    let mut b = [0u8; 8];
                    b.copy_from_slice(&buf[*pos..*pos + 8]);
                    *pos += 8;
                    arr.push(i64::from_le_bytes(b));
                }
                RZTag::LongArray(arr)
            }
            _ => RZTag::Byte(0),
        }
    }

    fn read_var_u32(buf: &[u8], pos: &mut usize) -> u32 {
        let mut v = 0u32;
        let mut shift = 0u32;
        while *pos < buf.len() {
            let b = buf[*pos];
            *pos += 1;
            v |= u32::from(b & 0x7F) << shift;
            if (b & 0x80) == 0 {
                return v;
            }
            shift += 7;
        }
        v
    }

    fn read_var_int(buf: &[u8], pos: &mut usize) -> i32 {
        let v = Self::read_var_u32(buf, pos);
        ((v >> 1) as i32) ^ -((v & 1) as i32)
    }

    fn read_var_long(buf: &[u8], pos: &mut usize) -> i64 {
        let mut v = 0u64;
        let mut shift = 0u32;
        while *pos < buf.len() {
            let b = buf[*pos];
            *pos += 1;
            v |= u64::from(b & 0x7F) << shift;
            if (b & 0x80) == 0 {
                return ((v >> 1) as i64) ^ -((v & 1) as i64);
            }
            shift += 7;
        }
        0
    }

    fn read_string(buf: &[u8], pos: &mut usize) -> String {
        let n = Self::read_var_u32(buf, pos) as usize;
        let slice = &buf[*pos..*pos + n];
        *pos += n;
        String::from_utf8_lossy(slice).into_owned()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RZBlob {
    data: Arc<Vec<u8>>,
    hash: u64,
}

impl Serialize for RZBlob {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.data.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RZBlob {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = Vec::<u8>::deserialize(deserializer)?;
        Ok(Self::from_raw(raw))
    }
}

static INTERN_POOL: OnceLock<Mutex<HashMap<u64, Arc<Vec<u8>>>>> = OnceLock::new();

fn intern_pool() -> &'static Mutex<HashMap<u64, Arc<Vec<u8>>>> {
    INTERN_POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

impl RZBlob {
    pub fn empty() -> Self {
        Self {
            data: Arc::new(vec![0]),
            hash: 0,
        }
    }

    pub fn from_raw(enc: Vec<u8>) -> Self {
        if enc.is_empty() || enc == [0] {
            return Self::empty();
        }
        let h = Self::xx64(&enc);
        let mut pool = intern_pool().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = pool.get(&h)
            && existing.as_slice() == enc.as_slice()
        {
            return Self {
                data: Arc::clone(existing),
                hash: h,
            };
        }
        let arc = Arc::new(enc);
        pool.insert(h, Arc::clone(&arc));
        Self { data: arc, hash: h }
    }

    pub fn of(compound: &HashMap<String, RZTag>) -> Self {
        Self::from_raw(RZNbt::encode(compound))
    }

    pub fn to_compound(&self) -> HashMap<String, RZTag> {
        RZNbt::decode(&self.data)
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }

    pub fn clear_intern_pool() {
        let mut pool = intern_pool().lock().unwrap_or_else(|e| e.into_inner());
        pool.clear();
    }

    fn xx64(data: &[u8]) -> u64 {
        let mut h = 0x9E37_79B9_7F4A_7C15u64 ^ (data.len() as u64);
        let mut i = 0;
        while i + 8 <= data.len() {
            let mut word = [0u8; 8];
            word.copy_from_slice(&data[i..i + 8]);
            let x = u64::from_le_bytes(word);
            h ^= Self::mix(x);
            h = h.rotate_left(27).wrapping_mul(0x100_0000_01B3);
            i += 8;
        }
        let mut tail = 0u64;
        let mut shift = 0;
        while i < data.len() {
            tail |= u64::from(data[i]) << shift;
            shift += 8;
            i += 1;
        }
        h ^= Self::mix(tail);
        Self::mix(h)
    }

    fn mix(mut z: u64) -> u64 {
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

pub struct RZBlobEncoder;

impl RZBlobEncoder {
    pub fn shutdown() {
        RZBlob::clear_intern_pool();
    }
}