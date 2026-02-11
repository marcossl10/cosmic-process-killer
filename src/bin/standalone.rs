// SPDX-License-Identifier: GPL-3.0

//! Standalone launcher for Process Killer
//! Can be launched with Ctrl+Shift+Esc or from terminal

use cosmic_applet_process_killer::standalone::StandaloneApp;

fn main() -> cosmic::iced::Result {
    // Initialize i18n
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    cosmic_applet_process_killer::i18n::init(&requested_languages);

    // Launch as a standalone window application
    cosmic::app::run::<StandaloneApp>(
        cosmic::app::Settings::default()
            .size_limits(cosmic::iced::Limits::NONE.min_width(600.0).min_height(400.0))
            .size(cosmic::iced::Size::new(800.0, 600.0)),
        (),
    )
}
