use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::process::Command;

use anyhow::Result;

pub(super) const KNOWN_NAMES: &[&str] = &["debian", "ubuntu", "linuxmint"];

pub(super) fn rename(new_name: &str) -> Result<()> {
    crate::log::debug!("using Debian renamer");

    fs::write("/etc/hostname", new_name)?;

    let _ = Command::new("hostnamectl")
        .arg("set-hostname")
        .arg(new_name)
        .status()?;
    let _ = Command::new("/bin/hostname")
        .arg(new_name)
        .status()?;

    if let Ok(file) = File::open("/etc/hosts") {
        let lines: Vec<String> = io::BufReader::new(file)
            .lines()
            .filter_map(Result::ok)
            .collect();

        let mut hosts = File::create("/etc/hosts")?;
        writeln!(hosts, "127.0.1.1\t{}", new_name)?;
        for l in lines {
            if l.starts_with("127.0.1.1") {
                continue; 
            }
            writeln!(hosts, "{}", l)?;
        }
    }

    Ok(())
}
