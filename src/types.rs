use ocl::{
    prm::{cl_uchar, cl_uint},
    OclPrm,
};

pub const SCORE_DATA_SIZE: usize = 20;

pub type ScoreData = [u8; SCORE_DATA_SIZE];

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MpNumber {
    pub d: [cl_uint; 8],
}

unsafe impl OclPrm for MpNumber {}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point {
    pub x: MpNumber,
    pub y: MpNumber,
}

unsafe impl OclPrm for Point {}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HashResult {
    pub found: cl_uint,
    pub found_id: cl_uint,
    pub found_hash: [cl_uchar; 20],
}

unsafe impl OclPrm for HashResult {}

impl HashResult {
    pub fn new() -> Self {
        HashResult {
            found: 0,
            found_id: 0,
            found_hash: [0; 20],
        }
    }
}
