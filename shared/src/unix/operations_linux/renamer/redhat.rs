use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::process::Command;

use anyhow::Result;

pub(super) const KNOWN_NAMES: &[&str] = &["rhel", "redhat", "centos", "rocky", "alma", "fedora"];

pub(super) fn rename(new_name: &str) -> Result<()> {
    crate::log::debug!("using RH renamer");

    // 1. Escribir /etc/hostname
    fs::write("/etc/hostname", new_name)?;

    // 2. Forzar el nuevo hostname
    let _ = Command::new("hostnamectl")
        .arg("set-hostname")
        .arg(new_name)
        .status()?;
    let _ = Command::new("/bin/hostname")
        .arg(new_name)
        .status()?;

    // 3. Actualizar /etc/hosts
    if let Ok(file) = File::open("/etc/hosts") {
        let lines: Vec<String> = io::BufReader::new(file)
            .lines()
            .filter_map(Result::ok)
            .collect();

        let mut hosts = File::create("/etc/hosts")?;
        writeln!(hosts, "127.0.1.1\t{}", new_name)?;
        for l in lines {
            if !l.starts_with("127.0.1.1") {
                writeln!(hosts, "{}", l)?;
            }
        }
    }

    // 4. Actualizar /etc/sysconfig/network
    if let Ok(file) = File::open("/etc/sysconfig/network") {
        let lines: Vec<String> = io::BufReader::new(file)
            .lines()
            .filter_map(Result::ok)
            .collect();

        let mut net = File::create("/etc/sysconfig/network")?;
        writeln!(net, "HOSTNAME={}", new_name)?;
        for l in lines {
            if !l.starts_with("HOSTNAME") {
                writeln!(net, "{}", l)?;
            }
        }
    }

    Ok(())
}
