use std::path::PathBuf;

use tracing::error;
use winsafe::{self as w, co::ES, gui, prelude::*};

#[derive(Clone)]
#[allow(unused)]
pub struct ErrorWindow {
    wnd: gui::WindowMain,
    text_error: gui::Edit,
    btn_close: gui::Button,
    btn_config_file: Option<gui::Button>,

    error: String,
    config_file: Option<PathBuf>,
}

impl ErrorWindow {
    pub fn create_and_run(
        title: String,
        error: String,
        config_file: Option<PathBuf>,
    ) -> w::AnyResult<i32> {
        let wnd = gui::WindowMain::new(gui::WindowMainOpts {
            title: title.as_str(),
            size: gui::dpi(400, 400),
            ..Default::default()
        });

        let text_error = gui::Edit::new(
            &wnd,
            gui::EditOpts {
                text: error.as_str(),
                position: gui::dpi(8, 8),
                width: gui::dpi_x(400 - 16),
                height: gui::dpi_y(400 - 16 - 8 - 26),
                control_style: ES::MULTILINE | ES::WANTRETURN | ES::AUTOVSCROLL | ES::NOHIDESEL,
                resize_behavior: (gui::Horz::Resize, gui::Vert::Resize),
                ..Default::default()
            },
        );

        let btn_close = gui::Button::new(
            &wnd,
            gui::ButtonOpts {
                text: "&Close",
                position: gui::dpi(400 - 8 - 88, 400 - 8 - 26),
                ..Default::default()
            },
        );

        let btn_config_file = config_file.is_some().then(|| {
            gui::Button::new(
                &wnd,
                gui::ButtonOpts {
                    text: "&Open Config File",
                    width: gui::dpi_x(100),
                    position: gui::dpi(8, 400 - 8 - 26),
                    ..Default::default()
                },
            )
        });

        let new_self = Self {
            wnd,
            text_error,
            btn_close,
            btn_config_file,

            error,
            config_file,
        };
        new_self.events();

        new_self.wnd.run_main(None)
    }

    fn events(&self) {
        let text_error = self.text_error.clone();
        let error = self.error.clone();
        self.text_error.on().en_change(move || {
            text_error.set_text(error.as_str())?;
            Ok(())
        });
        let wnd = self.wnd.clone();
        self.btn_close.on().bn_clicked(move || {
            wnd.close();
            Ok(())
        });
        if let (Some(config_file), Some(btn_config_file)) =
            (self.config_file.clone(), self.btn_config_file.as_ref())
        {
            btn_config_file.on().bn_clicked(move || {
                if let Err(err) = open::that(&config_file) {
                    error!("Error opening config file: {}", err);
                }
                Ok(())
            });
        }
    }
}
