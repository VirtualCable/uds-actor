// Copyright (c) 2025 Virtual Cable S.L.U.
// All rights reserved.
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//    * Redistributions of source code must retain the above copyright notice,
//      this list of conditions and the following disclaimer.
//    * Redistributions in binary form must reproduce the above copyright notice,
//      this list of conditions and the following disclaimer in the documentation
//      and/or other materials provided with the distribution.
//    * Neither the name of Virtual Cable S.L.U. nor the names of its contributors
//      may be used to endorse or promote products derived from this software
//      without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
/*!
Author: Adolfo Gómez, dkmaster at dkmon dot com
*/
#![cfg_attr(not(test), windows_subsystem = "windows")]

use slint::ComponentHandle;

mod callbacks;
mod regcfg;

use shared::log;

slint::include_modules!();

fn main() {
    log::setup_logging("debug", shared::log::LogType::Config);

    // On debug builds, skip the admin check
    #[cfg(not(debug_assertions))]
    {
        let operations = shared::system::new_system();
        if operations.check_permissions().is_err() {
            let _ = rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Permission Error")
                .set_description("This program must be run with administrator privileges")
                .show();
            std::process::exit(1);
        }
    }

    let ui = AppWindow::new().unwrap();

    // Set some defaults
    ui.set_active_log_level(1);

    // Callbacks
    let ui_handle = ui.as_weak();
    ui.on_save_clicked(move || {
        if let Some(ui) = ui_handle.upgrade() {
            callbacks::bnt_save_clicked(&ui);
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_test_clicked(move || {
        if let Some(ui) = ui_handle.upgrade() {
            callbacks::btn_test_clicked(&ui);
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_close_clicked(move || {
        if let Some(ui) = ui_handle.upgrade() {
            log::debug!("Close button clicked, quitting");
            ui.hide().unwrap();
        }
    });

    // Validate ciphers
    ui.on_validate_ciphers(|ciphers| {
        if ciphers.is_empty() {
            return true;
        }
        for part in ciphers.split(':') {
            if part.trim().is_empty() {
                return false;
            }
        }
        true
    });

    // Fill the fields from existing config
    regcfg::fill_window_fields(&ui);

    ui.run().unwrap();
}
