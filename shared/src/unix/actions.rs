use crate::log;

pub async fn logoff() -> anyhow::Result<()> {
    log::info!("Logoff requested (stub)");
    // TODO: Close user session in Windows
    Ok(())
}

pub async fn screenshot() -> anyhow::Result<Vec<u8>> {
    log::info!("Screenshot requested (stub)");
    // TODO: Take windows screenshot
    // 1x1 transparent PNG (RGBA)
    // PNG file bytes: 89 50 4E 47 0D 0A 1A 0A ...
    // This is a minimal valid PNG for a 1x1 transparent pixel
    const PNG_1X1_TRANSPARENT: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
        0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41,
        0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
        0x42, 0x60, 0x82
    ];
    Ok(PNG_1X1_TRANSPARENT.to_vec())
}

pub async fn run_script(_script: &str) -> anyhow::Result<String> {
    // TODO: Execute script in Windows (maybe lua??). Maybe never implement this. :)
    Ok("".to_string())
}

pub async fn show_message(message: &str) -> anyhow::Result<String> {
    log::info!("Show message requested: {}", message);
    // TODO: Show message in Windows
    Ok("not implemented".to_string())
}
