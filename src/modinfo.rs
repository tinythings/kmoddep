use std::io::prelude::*;
use std::io::BufReader;
use std::{fs::File, process};

/// ModInfo contains current live module information
#[derive(Debug)]
pub struct ModInfo {
    pub name: String,
    pub mem_size: usize,
    pub mem_offset: usize, // Available for root only
    pub instances: u8,
    pub dependencies: Vec<String>,
}

/// lsmod is just parse /proc/modules
pub fn lsmod() -> Vec<ModInfo> {
    let mut curr_mods: Vec<ModInfo> = vec![];
    let rfe = File::open("/proc/modules");
    if rfe.is_err() {
        process::exit(1);
    }

    for rfe in BufReader::new(rfe.unwrap()).lines() {
        if rfe.is_err() {
            process::exit(1);
        }

        let mod_data: Vec<String> = rfe.unwrap().split(' ').map(str::to_string).collect();

        if mod_data.len() != 6 {
            process::exit(1);
        }

        curr_mods.push(ModInfo {
            name: mod_data[0].to_owned(),
            mem_size: mod_data[1].parse::<usize>().unwrap(),
            instances: mod_data[2].parse::<u8>().unwrap(),
            dependencies: if mod_data[3] == "-" {
                vec![]
            } else {
                mod_data[3]
                    .strip_suffix(',')
                    .unwrap()
                    .split(',')
                    .map(str::to_string)
                    .collect()
            },
            mem_offset: usize::from_str_radix(mod_data[5].strip_prefix("0x").unwrap(), 0x10)
                .unwrap(),
        });
    }

    curr_mods
}
