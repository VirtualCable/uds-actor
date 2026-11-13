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
Author: Janier Rodríguez, jrodriguez at virtualcable dot es
Author: Andres Schumann, aschumann at virtualcable dot es
*/
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use shared::log;

mod app;
mod callbacks;
mod regcfg;

#[tokio::main]
async fn main() -> eframe::Result {
    log::setup_logging("debug", shared::log::LogType::Config);

    let operations = shared::system::new_system();

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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("UDS Actor Configuration")
            .with_inner_size([520.0, 560.0])
            .with_min_inner_size([500.0, 520.0])
            .with_icon(gui_kit::logo::window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "UDS Actor Configuration",
        options,
        Box::new(move |cc| {
            gui_kit::theme::apply(&cc.egui_ctx);
            let mut fields = app::Fields::default();
            let test_enabled = regcfg::fill_fields(&mut fields);
            Ok(Box::new(app::ConfigApp::new(
                &cc.egui_ctx,
                operations,
                fields,
                test_enabled,
            )))
        }),
    )
}
