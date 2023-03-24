use ocl::prm::cl_uchar;
use std::fmt;

use crate::{
    types::{ScoreData, SCORE_DATA_SIZE},
    utils::to_hex_char,
};

pub const DEFAULT_PROFANITY_INVERSE_SIZE: usize = 255;
pub const DEFAULT_PROFANITY_INVERSE_MULTIPLIER: usize = 16384;
pub const DEFAULT_PROFANITY_LOCAL_WORK_SIZE: usize = 64;

const SPEED_METER_SAMPLE_COUNT: usize = 40;
const PROFANITY_MAX_SCORE: usize = 40;

pub struct Config {
    pub target: HashTarget,
    pub mode: Mode,
    pub public_key: [u8; 64],
    pub inverse_size: usize,
    pub inverse_multiplier: usize,
    pub compact_speed: bool,
    pub local_work_size: usize,
    pub max_work_size: usize,
}

impl Config {
    pub fn new(
        target: HashTarget,
        mode: Mode,
        public_key: [u8; 64],
        compact_speed: bool,
        inveser_size: usize,
        inverse_multiplier: usize,
        local_work_size: usize,
        max_work_size: usize,
    ) -> Self {
        Config {
            target,
            mode,
            public_key,
            compact_speed,
            inverse_size: inveser_size,
            inverse_multiplier,
            local_work_size,
            max_work_size: if max_work_size == 0 {
                inveser_size * inverse_multiplier
            } else {
                max_work_size
            },
        }
    }

    pub fn get_speed_meter_sample_count(&self) -> usize {
        SPEED_METER_SAMPLE_COUNT
    }

    pub fn get_max_score(&self) -> usize {
        PROFANITY_MAX_SCORE
    }
}

pub enum HashTarget {
    Address,
    Contract,
}

impl fmt::Display for HashTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashTarget::Address => write!(f, "Address"),
            HashTarget::Contract => write!(f, "Contract"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Mode {
    Benchmark,
    Doubles,
    Leading(u8),
    LeadingRange { min: u8, max: u8 },
    Letters,
    Matching(Vec<u8>),
    Mirror,
    Numbers,
    Range { min: u8, max: u8 },
    Zeros,
}

fn data_for_range_mode(min: u8, max: u8) -> (Option<ScoreData>, Option<ScoreData>) {
    let (mut data1, mut data2) = (ScoreData::default(), ScoreData::default());
    (data1[0], data2[0]) = (min as cl_uchar, max as cl_uchar);
    (Some(data1), Some(data2))
}

impl Mode {
    pub fn get_kernel_name(&self) -> &str {
        match self {
            Mode::Benchmark => "profanity_score_benchmark",
            Mode::Doubles => "profanity_score_doubles",
            Mode::Leading(_) => "profanity_score_leading",
            Mode::LeadingRange { .. } => "profanity_score_leadingrange",
            Mode::Letters => "profanity_score_range",
            Mode::Matching(_) => "profanity_score_matching",
            Mode::Mirror => "profanity_score_mirror",
            Mode::Numbers => "profanity_score_range",
            Mode::Range { .. } => "profanity_score_range",
            Mode::Zeros => "profanity_score_range",
        }
    }

    pub fn get_data(&self) -> (Option<ScoreData>, Option<ScoreData>) {
        return match self {
            Mode::Leading(char) => {
                let mut data1 = ScoreData::default();
                data1[0] = *char;
                (Some(data1), None)
            }
            Mode::Benchmark => (None, None),
            Mode::Doubles => (None, None),
            Mode::LeadingRange { min, max } => data_for_range_mode(*min, *max),
            Mode::Letters => data_for_range_mode(10, 15),
            Mode::Matching(data) => {
                let (mut data1, mut data2) = (ScoreData::default(), ScoreData::default());

                if data.len() > SCORE_DATA_SIZE * 20 {
                    panic!("Matching data is too long");
                }

                let mut it = data.iter();
                let mut i = 0;

                loop {
                    match (it.next(), it.next()) {
                        (Some(high), Some(low)) => {
                            data1[i] = *high << 4 | low;
                            data2[i] = 0xFF;
                        }
                        (Some(high), None) => {
                            data1[i] = *high << 4;
                            data2[i] = 0xF0;
                            break;
                        }
                        (_, _) => break,
                    };

                    i += 1;
                }

                (Some(data1), Some(data2))
            }
            Mode::Mirror => (None, None),
            Mode::Numbers => data_for_range_mode(0, 9),
            Mode::Range { min, max } => data_for_range_mode(*min, *max),
            Mode::Zeros => data_for_range_mode(0, 0),
        };
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Benchmark => write!(f, "Benchmark"),
            Mode::Doubles => write!(f, "Doubles"),
            Mode::Leading(char) => write!(f, "Leading<{}>", to_hex_char(*char)),
            Mode::LeadingRange { min, max } => {
                write!(
                    f,
                    "Leading Range<{}-{}>",
                    to_hex_char(*min),
                    to_hex_char(*max)
                )
            }
            Mode::Letters => write!(f, "Letters"),
            Mode::Matching(data) => {
                let mut s = String::new();
                for byte in data {
                    s.push(to_hex_char(*byte));
                }
                write!(f, "Matching<{}>", s)
            }
            Mode::Mirror => write!(f, "Mirror"),
            Mode::Numbers => write!(f, "Numbers"),
            Mode::Range { min, max } => {
                write!(f, "Range<{}-{}>", to_hex_char(*min), to_hex_char(*max))
            }
            Mode::Zeros => write!(f, "Zeros"),
        }
    }
}
