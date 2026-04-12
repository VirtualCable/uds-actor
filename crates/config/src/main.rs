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
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

mod callbacks;
mod regcfg;

use shared::log;

slint::include_modules!();

fn main() {
    log::setup_logging("debug", shared::log::LogType::Config);

    let operations = shared::system::new_system();

    // On debug builds, skip the admin check
    #[cfg(not(debug_assertions))]
    {
        if operations.check_permissions().is_err() {
            let _ = rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Permission Error")
                .set_description("This program must be run with administrator privileges")
                .show();
            std::process::exit(1);
        }
    }

    // Our auths list, on Arc to share between threads
    let auths = Arc::new(Mutex::new(
        Vec::<shared::broker::api::types::Authenticator>::new(),
    ));

    let ui = AppWindow::new().unwrap();

    // Set some defaults
    ui.set_active_authenticator(0);
    ui.set_active_log_level(1);

    // Callbacks
    let ui_handle = ui.as_weak();
    let saved_auths = auths.clone();
    ui.on_host_changed(move |host| {
        if let Some(ui) = ui_handle.upgrade() {
            log::debug!("Using UDS Server: {}", host);
            callbacks::uds_server_changed(&ui, saved_auths.clone());
        }
    });

    let ui_handle = ui.as_weak();
    let saved_auths = auths.clone();
    let ops = operations.clone();
    ui.on_register_clicked(move || {
        if let Some(ui) = ui_handle.upgrade() {
            // Fail if we can't get at least one network interface
            let interface = match ops.get_first_network_interface() {
                Ok(iface) => iface,
                Err(e) => {
                    log::error!("No network interfaces found: {}", e);
                    let _ = rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Error)
                        .set_title("Network Error")
                        .set_description("No network interfaces found, cannot continue")
                        .show();
                    return;
                }
            };
            callbacks::btn_register_clicked(&ui, saved_auths.clone(), ops.clone(), &interface);
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

    // Browse callbacks
    let ui_handle = ui.as_weak();
    ui.on_browse_preconnect_clicked(move || {
        if let Some(ui) = ui_handle.upgrade()
            && let Some(path) = rfd::FileDialog::new().pick_file()
        {
            ui.set_preconnect_cmd(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_browse_runonce_clicked(move || {
        if let Some(ui) = ui_handle.upgrade()
            && let Some(path) = rfd::FileDialog::new().pick_file()
        {
            ui.set_runonce_cmd(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_browse_postconfig_clicked(move || {
        if let Some(ui) = ui_handle.upgrade()
            && let Some(path) = rfd::FileDialog::new().pick_file()
        {
            ui.set_postconfig_cmd(path.to_string_lossy().to_string().into());
        }
    });

    // Validate ciphers (pure callback)
    ui.on_validate_ciphers(|ciphers| {
        if ciphers.is_empty() {
            return true;
        }
        // Simple check: split by colon and ensure no empty parts
        for part in ciphers.split(':') {
            if part.trim().is_empty() {
                return false;
            }
        }
        true
    });

    // Fill the fields from existing config
    regcfg::fill_window_fields(&ui);

    // Trigger initial auth query
    callbacks::uds_server_changed(&ui, auths.clone());

    ui.run().unwrap();
}
