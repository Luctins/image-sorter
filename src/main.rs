//! # File sorter

#![allow(unused_imports)]

/*--- Imports ------------------------------------------------------------------------------------*/

use std::{
    io::{self, prelude::*, Error},
    fs::{self, ReadDir},
    collections::{HashSet, HashMap},
    path::{PathBuf, Path},
    sync::Arc
};

pub use nannou::prelude::*;

pub use nannou_egui::{
    self,
    egui::{self, Response, TextBuffer, TextEdit, color::rgb_from_hsv},
    Egui,
};

use serde::{Deserialize, Serialize};

use clap::Parser;

/*--- Mod ----------------------------------------------------------------------------------------*/

mod data_store;

mod config;
use config::Config;

mod image_manager;
use image_manager::ImageManager;

mod text_suggest;

/*--- Global Constants ---------------------------------------------------------------------------*/

const CONFIG_FILE_NAME: &'static str = ".image-sorter.yaml";
pub const DEFAULT_CONFIG_S: &'static str = include_str!("../default/config.yaml");
pub const TAG_SEPARATOR: &'static str = "--";

lazy_static::lazy_static!{
    static ref DEFAULT_CONFIG: Config = serde_yaml::from_str(DEFAULT_CONFIG_S)
        .expect("failed to parse default configuration");

    pub static ref PLACEHOLDER_BUF: &'static [u8] =
        std::include_bytes!("../assets/placeholder.bmp");
}

/*--- Args ---------------------------------------------------------------------------------------*/

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg()]
    folder: PathBuf,
}

impl From<PathBuf> for Args {
    fn from(value: PathBuf) -> Self {
        Self { folder: value.into() }
    }
}

/*--- Model --------------------------------------------------------------------------------------*/

// model
structstruck::strike!{
    /// Main user state for the application
    struct Model {
        egui: Egui,

        config: Config,

        image_manager: ImageManager,

        state:
        #[derive(Debug, Clone, Copy, Default)]
        pub enum State {
            #[default]
            Idle,
            Input,
        },

        ui_fields:
        #[derive(Default)]
        pub struct {
            destination_filename: String,
            new_category: String,
            new_tag: String,
        },
    }
}

impl Model {
    pub fn new(_app: &App, egui: Egui) -> Self {
        use clap::error::{self, ErrorKind, Error};

        let args =
        match Args::try_parse() {
            Ok(a) => a,
            Err(e) => {
                Args::from(
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
                            panic!("unsupported");
                        }
                    })
            }
        };

        let cfg_path = &args.folder;

        // load configuration or default value
        let config = {
            let p = cfg_path.join(CONFIG_FILE_NAME);

            let cfg_str = fs::read_to_string(&p)
                .map_err(|e| {
                    // TODO: copy default config to the cwd
                    println!("no local config present at {p:?}, {e}; ");
                    println!("creating config file '{cfg_path:?}'");

                    let _ = std::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(&cfg_path)
                        .map(|mut f|
                             f.write(DEFAULT_CONFIG_S.as_bytes())
                             .map_err(|e| eprintln!("failed to write {cfg_path:?}: {e}"))
                        )
                        .map_err(|e| eprintln!("failed to create {cfg_path:?} : {e}"));

                    e
                })
                .unwrap_or(DEFAULT_CONFIG_S.to_string());

            let c: Config = serde_json::from_str(&cfg_str).unwrap_or(DEFAULT_CONFIG.clone());

            // c.buttons = {
            //     let mut tmp = c.buttons.drain().collect::<Vec<_>>();
            //     tmp.sort_by(|prev, next| prev.0.cmp(&next.0));
            //     tmp.into_iter().collect()
            // };

            // // make sure the trash button exists
            // _ = c.buttons.insert(
            //     "k".to_string(),
            //     ButtonConfig {
            //         button_label: "\u{1F5D1}".to_string(), // wastebasket symbol
            //         label: "Trash".to_string(),
            //         path: "trash".to_string(),
            //     });

            // c.buttons.iter_mut().for_each(|(b_id, b_cfg)| {
            //     let old = b_cfg.label.clone();

            //     let (up, low) = {
            //         let c = b_id.chars().next().unwrap();
            //         (c.to_ascii_lowercase(), c.to_ascii_uppercase())
            //     };


            //     let new =  {
            //         let new = old.replacen(low, format!("[{}]", low).as_str(), 1);
            //         if new == old {
            //             old.replacen(up, format!("[{}]", up).as_str(), 1)
            //         } else {
            //             new
            //         }
            //     };

            //     b_cfg.label.replace(
            //         if new == old {
            //             format!("{}: [{}]", old, b_id)
            //         } else {
            //             new
            //         }.as_str()
            //     );
            // });

            // TODO: implement keyboard shortcuts first or some kind
            // of 'fast sort mode' with single key commands (vim-like)
            // for (id, button) in &mut c.buttons {
            //     button.label.replace(&button.label.replacen(id, &format!("[{}]", id), 1));
            // }

            println!("configuration: {c:?}");
            c
        };

        Model {
            image_manager: ImageManager::new(&args.folder, &config),

            // init to default
            ui_fields: Default::default(),
            state: Default::default(),

            config,
            egui,
        }

    }

    pub fn add_category(&mut self, new_category: &str) {
        self.config.categories.insert(new_category.to_string());

        println!("added new category: {}", new_category);

        todo!()
        //std::fs::write(&self.config_path, &cfg_str).expect("cannot write preferences");
    }
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

/// Window and GUI update fn
fn update(app: &App, model: &mut Model, update: Update) {
    let egui = &mut model.egui;
    let manager = &mut model.image_manager;

    let config = &model.config;
    let filename_buff = &mut model.ui_fields.destination_filename;

    let mut pos = manager.image_index as f32;
    let max_img = (manager.get_images_len() - 1) as f32;

    egui.set_elapsed_time(update.since_start);
    let egui_context = egui.begin_frame();

    //ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {

    // GUI layout
    egui::TopBottomPanel::bottom("File Control").show(&egui_context, |ui| {

        // Command detection
        // {
            // let input_state = ui.input();

        //     for ev in &input_state.raw.events {
        //         match ev {
        //             egui::Event::Key {
        //                 key,
        //                 pressed: true,
        //                 modifiers: egui::Modifiers { command, ..},
        //             } => {
        //                 // fixed delete key
        //                 match key {
        //                     // egui::Key::Delete => {
        //                     //     let btn_cfg = config.buttons.get("t").unwrap();
        //                     //     manager.move_current(&btn_cfg.path, filename_buff);
        //                     // },

        //                     // egui::Key::ArrowDown | egui::Key::ArrowLeft => {
        //                     //     manager.prev_image();
        //                     // },

        //                     // egui::Key::ArrowUp | egui::Key::ArrowRight => {
        //                     //     manager.next_image();
        //                     // },

        //                     _ => {
        //                         if *command {
        //                             let id:&str = dbg!(stringify!(key));
        //                             if let Some(btn_cfg) = config.buttons.get(id) {
        //                                 manager.move_current(&btn_cfg.path, filename_buff);
        //                             }
        //                         }
        //                     }
        //                 }

        //             },
        //             _ => {},
        //         }
        //     }
        // }

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

        let create_movement_buttons = |col: &mut [egui::Ui]| {
            {
                let c_ui = &mut col[0];
                c_ui.label("Prev");
                let btn = c_ui.add_enabled(manager.image_index != 0, egui::Button::new(" ⏴ "));
                if btn.clicked() {
                    manager.prev_image();
                }

                // c_ui.label("Trash");
                // let btn = c_ui.button("  \u{1F5D1}  "); // TODO: read btn state
                // if btn.clicked() {
                //     manager.move_current(image_manager::dir::TRASH, "trashed");
                //     filename_buff.clear();
                // }
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

                // c_ui.label("Separate");
                // let btn = c_ui.add(egui::Button::new(" \u{1F4E4} "));
                // if btn.clicked() {
                //     manager.move_current(image_manager::dir::OTHER, filename_buff);
                //     filename_buff.clear();
                // }
            }
        };

        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.columns(2, create_movement_buttons)
        });

        let create_buttons = |col: &mut [egui::Ui]| {
             {
                 let mut pos = 0;
                 // TODO: add keyboard shortcuts using C - 'button ids'
                 // TODO: allow for reordering buttons with a configuration field
                 // (currently based hashmap keys order afaik)
                 for (_button_id, button_cfg) in &config.buttons {
                     let c_ui = &mut col[pos];
                     pos += 1;
                     c_ui.label(button_cfg.label.as_str());

                     let btn = c_ui.button(format!("  {}  ", button_cfg.button_label));
                     if btn.clicked() {
                         manager.move_current(&button_cfg.path, filename_buff);
                     }
                 }
            }
        };

        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.columns(config.buttons.len(), create_buttons)
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
            egui::TextEdit::singleline(filename_buff)
                .code_editor()
                .hint_text("New filename")
                .lock_focus(true), //.cursor_at_end(true)
        );

        let seg_buff = filename_buff.clone();
        let mut segments = text_suggest::get_segments(&seg_buff);

         let suggestions: Vec<String> = {
             if let Some(segment) = segments.last() {
                 todo!("get suggestions and check if first position to not suggest tags");
                // text_suggest:: .get_suggestions(&segment)
            } else {
                vec![]
            }
         };


        //     // let popup_id = ui.make_persistent_id("suggestions_box");

        //     // if suggestions.first().is_some() {
        //     //     ui.memory().toggle_popup(popup_id);
        //     // }

        //     // egui::popup_below_widget(ui, popup_id, &inputbox_r, |ui| {
        //     //     for sug in &suggestions {
        //     //         ui.label(sug);
        //     //     }
        //     // });


        let k = ui.input();

        if inputbox_r.lost_focus() {
            if k.key_pressed(egui::Key::Enter) {
                manager.move_current(&config.default_folder, filename_buff);
                filename_buff.clear();
                inputbox_r.request_focus();
            }
        }

        // tab pressed
        if k.key_released(egui::Key::Tab) {

            if let (Some(replacement), Some(dest)) =
                (suggestions.first(), segments.last_mut())
            {
                *dest = replacement;

                // get the new segment and cycle try cycling between options
                // if let Some(ref mut sel) = dbg!(text_suggest.current_selection) {
                //     if let Some(sug_next) = &suggestions.get(*sel + 1) {
                //         *sel += 1;
                //         sug_next
                //     } else {
                //         dbg!(*sel);
                //         *sel = 0;
                //         replacement
                //     }
                // } else {
                //     text_suggest.current_selection = Some(0);
                //     replacement
                // };

                println!("completion: {replacement:?}");

                // replace name with new string
                *filename_buff =
                    segments.iter().fold(String::new(), |mut acc, part| {
                        acc.push_str(part);
                        acc.push_str(text_suggest::SEPARATOR);
                        acc
                    });
            }
        }

        let mut suggestions_iter = suggestions.iter();
        let first: String = suggestions_iter
            .next()
            .map(|i| i.clone())
            .unwrap_or(" ".to_string());

        ui.label(
            format!("Suggestions: {}",
            suggestions_iter.take(2)
                    .fold(first, |mut res, item| {
                        res.push_str(", ");
                        res.push_str(item);
                        res
                    })
            ));

        // new category box
        ui.separator();
        ui.separator();
        ui.label("Add new category");
        ui.columns(2, |col| {

            // set the input buffer as always the last segment if it exists
            if let Some(segment) = segments.last() {
                model.ui_fields.new_category.replace(segment);
            }

            col[0].text_edit_singleline(&mut model.ui_fields.new_category);
            if col[1].button(" \u{002b} ").clicked() {
                todo!("add categories")
                // model.add_category(&model.ui_fields.new_category);
                // model.ui_fields.new_category.clear();
                //text_suggest.add_category();
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

    let img_texture = model.image_manager.get_texture();

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
