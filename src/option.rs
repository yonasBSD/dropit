use std::path::PathBuf;
use structopt::StructOpt;
use crate::upload::expiration::Threshold;
use std::net::IpAddr;
use byte_unit::{ByteError, Byte};

#[derive(StructOpt, Debug)]
pub struct Options {
    #[structopt(short = "u", long, default_value = "uploads")]
    pub uploads_dir: PathBuf,
    #[structopt(short = "a", long, default_value = "127.0.0.1")]
    pub address: IpAddr,
    #[structopt(short = "p", long, default_value = "8080")]
    pub port: u16,
    #[structopt(short = "R", long = "behind-reverse-proxy")]
    pub behind_proxy: bool,
    #[structopt(short = "t", long = "threshold", required = true)]
    pub thresholds: Vec<Threshold>,
    #[structopt(short = "s", long, required = true, parse(try_from_str = parse_size))]
    pub ip_size_sum: u64,
    #[structopt(short = "c", long, required = true)]
    pub ip_file_count: usize,
    #[structopt(short = "S", long, required = true, parse(try_from_str = parse_size))]
    pub global_size_sum: u64,
    #[structopt(short = "C", long, default_value = "#15b154")]
    pub color: String,
}

fn parse_size(s: &str) -> Result<u64, ByteError> {
    Ok(s.parse::<Byte>()?.get_bytes())
}