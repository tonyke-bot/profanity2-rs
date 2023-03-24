use clap::{command, Parser, Subcommand, ValueEnum};
use std::collections::HashSet;

use crate::{
    config::{
        Config, HashTarget, Mode, DEFAULT_PROFANITY_INVERSE_MULTIPLIER,
        DEFAULT_PROFANITY_INVERSE_SIZE, DEFAULT_PROFANITY_LOCAL_WORK_SIZE,
    },
    utils::{hex_char_to_u8, hex_char_to_u8_unchecked, hex_str_to_bytes, is_hex_char},
};

#[derive(Clone, ValueEnum)]
pub enum Target {
    Address,
    Contract,
}

impl Target {
    fn to_hash_target(&self) -> HashTarget {
        match self {
            Target::Address => HashTarget::Address,
            Target::Contract => HashTarget::Contract,
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(
        short,
        long,
        required = true,
        help = "Set seed to use for address generation.",
        value_parser = seed_validator,
    )]
    pub seed: String,

    #[arg(
        long,
        use_value_delimiter = true,
        help = "Skip devices with given indices (comma separated)."
    )]
    pub skip_devices: Option<Vec<usize>>,

    #[arg(
        long = "work-max",
        short = 'W',
        help = "Set OpenCL maximum work size. Default to [-i * -I].",
        default_value_t = DEFAULT_PROFANITY_INVERSE_SIZE * DEFAULT_PROFANITY_INVERSE_MULTIPLIER,
    )]
    pub max_work_size: usize,

    #[arg(
        long = "work",
        short = 'w',
        help = "Set OpenCL local work size.",
        default_value_t = DEFAULT_PROFANITY_LOCAL_WORK_SIZE,
    )]
    pub work_size: usize,

    #[arg(
        long,
        short = 'i',
        help = "Set size of modular inverses to calculate in one work item.",
        default_value_t = DEFAULT_PROFANITY_INVERSE_SIZE,
    )]
    pub inverse_size: usize,

    #[arg(
        long,
        short = 'I',
        help = "Set how many above work items will run in parallell.",
        default_value_t = DEFAULT_PROFANITY_INVERSE_MULTIPLIER,
    )]
    pub inverse_multiplier: usize,

    #[arg(
        long,
        help = "Only show total iteration speed.",
        default_value_t = false
    )]
    pub compact_speed: bool,

    #[arg(
        long,
        short,
        value_enum,
        help = "Set target to search for",
        default_value_t = Target::Address,
    )]
    pub target: Target,

    #[command(subcommand)]
    pub command: ModeCommands,
}

impl Cli {
    pub fn get_seed(&self) -> [u8; 64] {
        hex_str_to_bytes(&self.seed).as_slice().try_into().unwrap()
    }

    pub fn to_config(&self) -> Config {
        Config::new(
            self.target.to_hash_target(),
            self.command.to_mode(),
            self.get_seed(),
            self.compact_speed,
            self.inverse_size,
            self.inverse_multiplier,
            self.work_size,
            self.max_work_size,
        )
    }

    pub fn get_skip_devices(&self) -> HashSet<usize> {
        self.skip_devices
            .as_ref()
            .map(|v| v.iter().cloned().collect())
            .unwrap_or_default()
    }
}

#[derive(Subcommand)]
pub enum ModeCommands {
    #[command(about = "Run without any scoring, a benchmark.")]
    Benchmark,

    #[command(about = "Score on hashes leading with hexadecimal pairs.")]
    Doubles,

    #[command(about = "Score on hashes leading with given hex character.")]
    Leading {
        #[arg(value_parser = hex_char_parser)]
        leading: u8,
    },

    #[command(about = "Score on hashes leading with characters within given range.")]
    LeadingRange {
        #[arg(value_parser = hex_char_parser)]
        min: u8,
        #[arg(value_parser = hex_char_parser)]
        max: u8,
    },

    #[command(about = "Score on letters anywhere in hash.")]
    Letters,

    #[command(about = "Score on hashes matching given hex string.")]
    Matching {
        #[arg(value_parser = hex_str_validator)]
        m: String,
    },

    #[command(about = "Score on mirroring from center.")]
    Mirror,

    #[command(about = "Score on numbers anywhere in hash.")]
    Numbers,

    #[command(about = "Score on hashes having characters within given range anywhere.")]
    Range {
        #[arg(value_parser = hex_char_parser)]
        min: u8,
        #[arg(value_parser = hex_char_parser)]
        max: u8,
    },

    #[command(about = "Score on zeros anywhere in hash.")]
    Zeros,
}

impl ModeCommands {
    pub fn to_mode(&self) -> Mode {
        match self {
            ModeCommands::Benchmark => Mode::Benchmark,
            ModeCommands::Doubles => Mode::Doubles,
            ModeCommands::Leading { leading } => Mode::Leading(*leading),
            ModeCommands::LeadingRange { min, max } => Mode::LeadingRange {
                min: *min,
                max: *max,
            },
            ModeCommands::Letters => Mode::Letters,
            ModeCommands::Matching { m } => {
                Mode::Matching(hex_char_vec_parser_unchecked(m.as_str()))
            }
            ModeCommands::Mirror => Mode::Mirror,
            ModeCommands::Numbers => Mode::Numbers,
            ModeCommands::Range { min, max } => Mode::Range {
                min: *min,
                max: *max,
            },
            ModeCommands::Zeros => Mode::Zeros,
        }
    }
}

fn seed_validator(s: &str) -> Result<String, String> {
    let mut bytes = s.as_bytes();

    if bytes.len() > 2 && bytes[0] == '0' as u8 && bytes[1] == 'x' as u8 {
        bytes = &bytes[2..];
    }

    if bytes.len() != 128 {
        return Err(format!("`{s}` should be 128 characters long").to_string());
    }

    bytes
        .iter()
        .all(|v| is_hex_char(*v as char))
        .then(|| s.to_string())
        .ok_or(format!("`{s}` isn't a valid seed").to_string())
}

fn hex_char_parser(s: &str) -> Result<u8, String> {
    s.chars()
        .nth(0)
        .and_then(|v| hex_char_to_u8(v))
        .ok_or(format!("`{s}` isn't a valid hex character").to_string())
}

fn hex_str_validator(s: &str) -> Result<String, String> {
    if s.len() > 40 {
        return Err(format!("`{s}` is too long").to_string());
    }

    s.chars()
        .all(|v| is_hex_char(v))
        .then(|| s.to_string())
        .ok_or(format!("`{s}` isn't a valid hex string").to_string())
}

fn hex_char_vec_parser_unchecked(s: &str) -> Vec<u8> {
    s.chars().map(|v| hex_char_to_u8_unchecked(v)).collect()
}

pub fn parse_cli() -> Cli {
    Cli::parse()
}
