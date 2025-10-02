use std::collections::HashMap;
use std::sync::OnceLock;

use anyhow::Result;

mod alt;
mod debian;
mod opensuse;
mod redhat;

static RENAMERS: OnceLock<HashMap<&'static str, fn(&str) -> Result<()>>> = OnceLock::new();

pub(super) fn renamer(new_name: &str, os_name: &str) -> Result<()> {
    let renamer = RENAMERS.get_or_init(|| {
        let mut m = HashMap::new();
        for (fnc, known_names) in &[
            (alt::rename as fn(&str) -> Result<()>, alt::KNOWN_NAMES),
            (
                debian::rename as fn(&str) -> Result<()>,
                debian::KNOWN_NAMES,
            ),
            (
                opensuse::rename as fn(&str) -> Result<()>,
                opensuse::KNOWN_NAMES,
            ),
            (
                redhat::rename as fn(&str) -> Result<()>,
                redhat::KNOWN_NAMES,
            ),
        ] {
            for &name in *known_names {
                m.insert(name, *fnc);
            }
        }
        m
    });

    // Search for a renamer
    for (key, func) in renamer.iter() {
        if os_name.contains(key) {
            return func(new_name);
        }
    }

    // Use debian renamer as fallback
    debian::rename(new_name)
}
