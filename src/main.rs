//! # File sorter

/*--- Imports ------------------------------------------------------------------------------------*/

#![allow(unused_imports)]
use std::{collections::{HashSet, HashMap}, fs::ReadDir, path::PathBuf, sync::Arc};

pub use nannou::prelude::*;
pub use nannou_egui::{
    self,
    egui::{self, Response, TextBuffer, TextEdit, color::rgb_from_hsv},
    Egui,
};


use serde::{Deserialize, Serialize};

use clap::Parser;

mod image_manager;
use image_manager::ImageManager;

mod text_suggest;
use text_suggest::TextSuggester;

/*--- Global Constants ---------------------------------------------------------------------------*/

/*--- Impl ---------------------------------------------------------------------------------------*/


/// Main user state for the application
struct Model {
    egui: Egui,
    text_suggest: TextSuggester,
    manager: ImageManager,
}
impl Model {
    pub fn new(app: &App, egui: Egui) -> Self {
        use clap::error::{self, ErrorKind, Error};

        let r = Args::try_parse();

        let folder = match r {
            Ok(a) => {
                a.folder
            },
            Err(ref e) => {
                match e.kind() {
                    ErrorKind::MissingRequiredArgument => {
                        println!("using current dir as fallback");
                        std::env::current_dir()
                            .expect("cannot read current dir but no args provided")
                    },

                    ErrorKind::DisplayHelp => {
                        println!("{e}");
                        std::process::exit(0);
                    },

                    _ => {
                        let _ = r.unwrap();
                        unreachable!("unsupported");
                    }
                }
            },
        };

        Model {
            text_suggest: TextSuggester::new(&app.assets_path().expect("cannot open project path")),
            manager: ImageManager::new(folder),
            egui,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg()]
    folder: PathBuf,
}

/*--- Main application ---------------------------------------------------------------------------*/

fn main() {
    nannou::app(model).update(update).run();
}

/// Init function
fn model(app: &App) -> Model {
    app.set_exit_on_escape(true);

    // Create window
    let window_id = app
        .new_window()
        .view(view)
        .raw_event(raw_window_event)
        .build()
        .unwrap();

    let window = app.window(window_id).unwrap();
    let egui = Egui::from_window(&window);

    Model::new(app, egui)
}

/// Window update fn
fn update(app: &App, model: &mut Model, update: Update) {
    let egui = &mut model.egui;
    let manager = &mut model.manager;
    let text_suggest = &mut model.text_suggest;

    let mut pos = manager.image_index as f32;
    let max_img = (manager.get_images_len() - 1) as f32;

    egui.set_elapsed_time(update.since_start);
    let egui_context = egui.begin_frame();

    // GUI layout
    egui::TopBottomPanel::bottom("File Control").show(&egui_context, |ui| {
        //ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {

        ui.label(format!("current image: {}", manager.get_current_filename()));

        ui.separator();
        ui.label("Controls");

        // Slider + open default btn
        ui.columns(2, |col| {
            let r = col[0].add(egui::Slider::new(&mut pos, 0.0..=max_img).text("Current Position"));
            if r.changed() {
                manager.seek_to_image(pos as usize);
            }

            if col[1].button("Open file in default program").clicked() {
                let _ = std::process::Command::new("xdg-open")
                    .arg(manager.get_current_path())
                    .spawn()
                    .unwrap();
            }
        });

        let create_buttons = |col: &mut [egui::Ui]| {
            {
                let c_ui = &mut col[0];
                c_ui.label("Prev");
                let btn = c_ui.add_enabled(manager.image_index != 0, egui::Button::new(" ⏴ "));
                if btn.clicked() {
                    manager.prev_image();
                }

                c_ui.label("Trash");
                let btn = c_ui.button("  \u{1F5D1}  "); // TODO: read btn state
                if btn.clicked() {
                    manager.move_current(image_manager::dir::TRASH, "trashed");
                }
            }

            {
                let c_ui = &mut col[1];
                c_ui.label("Next");
                let btn = c_ui.add_enabled(
                    manager.image_index != (manager.get_images_len() - 1),
                    egui::Button::new(" ⏵ "),
                );

                if btn.clicked() {
                    manager.next_image()
                }

                c_ui.label("Separate");
                let btn = c_ui.add(egui::Button::new(" \u{1F4E4} "));
                if btn.clicked() {
                    let name = manager.filename_buffer.clone();
                    manager.move_current(image_manager::dir::OTHER, &name);
                }
            }
        };
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.columns(2, create_buttons)
        });

        // Progress bar
        ui.separator();
        {
            ui.label("Remaining files");
            let total = manager.get_total_files();
            let current_total = manager.get_images_len();

            let p = 1.0 - ((current_total as f32)/(total as f32));
            ui.add(egui::ProgressBar::new(p).text(format!(
                "{} / {} - {:.2} %",
                total - current_total,
                total,
                p * 100.0
            )));
        }

        // Main input box
        ui.separator();
        ui.label("New file name:");
        let inputbox_r = ui.add(
            egui::TextEdit::singleline(&mut manager.filename_buffer)
                .code_editor()
                .hint_text("New filename")
                .lock_focus(true), //.cursor_at_end(true)
        );


        let mut segments = text_suggest::get_segments(&manager.filename_buffer);

        // let suggestions: Vec<String> = {
        //     // TODO: add constant for separator

        //     let suggestions = if let Some(segment) = segments.last() {
        //         text_suggest.get_suggestions(&segment)
        //     } else {
        //         vec![]
        //     };



        //     // let popup_id = ui.make_persistent_id("suggestions_box");

        //     // if suggestions.first().is_some() {
        //     //     ui.memory().toggle_popup(popup_id);
        //     // }

        //     // egui::popup_below_widget(ui, popup_id, &inputbox_r, |ui| {
        //     //     for sug in &suggestions {
        //     //         ui.label(sug);
        //     //     }
        //     // });


        //     // detect confirmation
        //     if let Some(k) = ui.input().keys_down.iter().next() {
        //         if text_suggest.last_key_changed(*k) {
        //             match k {
        //                 egui::Key::Enter => {
        //                     if inputbox_r.lost_focus() {
        //                         let name = manager.filename_buffer.clone();
        //                         manager.move_current(app, DIR_OUTPUT, &name);
        //                         inputbox_r.request_focus();
        //                     }
        //                 }
        //                 egui::Key::Tab => {
        //                     // both replacement and destination are not empty
        //                     if let (Some(replacement), Some(dest)) =
        //                         (suggestions.first(), segments.last_mut())
        //                     {
        //                         *dest = replacement.to_string();
        //                         println!("completion: {replacement:?}");

        //                         // replace name with new string
        //                         manager.filename_buffer =
        //                             segments.iter().peekable().fold(String::new(), |mut acc, part| {
        //                                 acc.push_str(part);
        //                                 acc.push_str("--");
        //                                 acc
        //                             });
        //                     }
        //                 }
        //                 _ => {}
        //             }
        //         }
        //     }
        //     suggestions
        // };

        // let mut suggestions_iter = suggestions.iter();
        // let first: String = suggestions_iter
        //     .next()
        //     .map(|i| i.clone())
        //     .unwrap_or(" ".to_string());

        // let _lab_h = ui.label(format!(
        //     "Suggestions: {}",
        //     suggestions_iter.take(2)
        //         .fold(first, |mut res, item| {
        //             res.push_str(", ");
        //             res.push_str(item);
        //             res
        //     })
        // ));

        // new category box
        ui.separator();
        ui.separator();
        ui.label("Add new category");
        ui.columns(2, |col| {

            // set the input buffer as always the last segment if it exists
            if let Some(segment) = segments.last() {
                text_suggest.new_category_buffer.replace(segment);
            }

            col[0].text_edit_singleline(&mut text_suggest.new_category_buffer);
            if col[1].button(" \u{002b} ").clicked() {
                text_suggest.add_category();
            }
        });
    });
    manager.update_texture(app);
}

/// Drawing loop
fn view(app: &App, model: &Model, frame: Frame) {
    const PAD: f32 = 45.0;

    let draw = app.draw();
    frame.clear(BLACK);

    let win = app.window_rect();
    let canvas = Rect::from(win.clone()).top_left_of(win).pad_bottom(300.0);

    let img_texture = model.manager.get_texture();

    let [img_w, img_h] = img_texture.size();

    #[allow(unused)]
    let mut dbg_text = String::new();

    // scale image preserving proportions
    let wh: Vec2 = {
        let img_w_f = img_w as f32;
        let img_h_f = img_h as f32;
        let img_h_fit = img_h_f * (canvas.w() / img_w_f);
        let img_w_fit = img_w_f * (canvas.h() / img_h_f);

        let fit_to_width = || {
            // fit to width
            Vec2::new(canvas.w() - PAD, img_h_fit - PAD)
        };

        let fit_to_height = || {
            // fit to height
            Vec2::new(img_w_fit - PAD, canvas.h() - PAD)
        };

        if img_w > img_h {
            if img_h_fit < canvas.h() {
                //dbg_text += "1".into();

                fit_to_width()
            } else {
                //dbg_text += "2".into();

                fit_to_height()
            }
        } else {
            if img_h_fit > canvas.h() {
                //dbg_text += "3".into();

                fit_to_height()
            } else {
                //dbg_text += "4".into();

                fit_to_width()
            }
        }
    };

    let xy = Point2::new(canvas.x(), canvas.y());

    //let view = model.manager.current_image.0.view().build();

    // bg rect
    draw.rect()
        //.xy(canvas.wh())
        .wh(canvas.wh())
        .xy(canvas.xy())
        .color(BLACK);

    draw.rect()
        .xy(xy)
        .wh(wh+Vec2::new(PAD, PAD))
        .color(DARKGREY);

    draw.texture(img_texture.as_ref())
        .xy(xy)
        .wh(wh);

    // draw.text(&dbg_text)
    //     .xy(canvas.mid_top())
    //     .font_size(25);

    // run queued drawing commands
    draw.to_frame(app, &frame).unwrap();

    // draw ui on top
    model.egui.draw_to_frame(&frame).unwrap();
}

/// Let egui handle things like keyboard and mouse input.
fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.egui.handle_raw_event(event);
}
