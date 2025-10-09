// uds-build/src/lib.rs
use chrono::{Datelike, Utc, NaiveDate};
use winres::WindowsResource;

pub fn build_windows(product_name: &str, description: &str, icon: Option<&str>, bmp: Option<&str>) {
    let icon = icon.unwrap_or("../img/uds.ico");
    let bmp = bmp.unwrap_or("../img/uds.bmp");
    println!("cargo:rerun-if-changed={icon}");
    println!("cargo:rerun-if-changed={bmp}");

    let current_year = Utc::now().year();
    let base_date = NaiveDate::from_ymd_opt(1972, 7, 1).unwrap();
    let today = Utc::now().date_naive();
    let build_days = (today - base_date).num_days();

    let (major, minor, patch, build) = (
        env!("CARGO_PKG_VERSION_MAJOR").parse::<u64>().unwrap(),
        env!("CARGO_PKG_VERSION_MINOR").parse::<u64>().unwrap(),
        env!("CARGO_PKG_VERSION_PATCH").parse::<u64>().unwrap(),
        build_days as u64,
    );

    let version: u64 = (major << 48) | (minor << 32) | (patch << 16) | build;

    let mut res = WindowsResource::new();
    res.set_icon(icon);

    res.set_version_info(winres::VersionInfo::FILEVERSION, version);
    res.set_version_info(winres::VersionInfo::PRODUCTVERSION, version);

    res.set_language(0x0409);

    res.set("FileVersion", &format!("{major}.{minor}.{patch}.{build}"));
    res.set("ProductVersion", &format!("{major}.{minor}.{patch}.{build}"));
    res.set("ProductName", product_name);
    res.set("FileDescription", description);
    res.set(
        "LegalCopyright",
        format!("Copyright © 2012-{current_year} Virtual Cable S.L.U.").as_str(),
    );
    res.set("CompanyName", "Virtual Cable S.L.U.");

    res.append_rc_content(&format!(r##"101 BITMAP DISCARDABLE "{}""##, bmp));

    res.compile().unwrap();
}
