//! # File sorter

/*--- Imports ------------------------------------------------------------------------------------*/

#![allow(unused_imports)]
use std::{path::PathBuf, fs::ReadDir, sync::Arc, collections::HashSet};

use nannou::prelude::*;
use nannou_egui::{self, egui::{self, TextEdit, TextBuffer, Response}, Egui};

use cached::{proc_macro::cached, SizedCache};

use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;

use clap::Parser;

/*--- Global Constants ---------------------------------------------------------------------------*/

const PLACEHOLDER_FILENAME: &str = "missing-image-placeholder.png";
const DIR_TRASH: &str = "trash";
static DIR_OUTPUT: &str = "output";
static DIR_OTHER: &str = "separate";

/*--- Impl ---------------------------------------------------------------------------------------*/

/// Image and file manager
struct ImageManager {
    total_files: usize,
    dir: PathBuf,
    new_name: String,
    images: Vec<String>,
    current_image: (wgpu::Texture, usize),

    // /// Image placeholder
    //placeholder: ImageBuffer<Rgb<u8>, Vec<u8>>,
}
impl ImageManager {
    fn new(app: &App, images_path: PathBuf) -> Self {
        let dir = images_path.clone();
        println!("images path: {dir:?}");

        for d in [DIR_OTHER, DIR_OUTPUT, DIR_TRASH] {
            std::fs::create_dir_all(dir.join("output").join(d))
                .expect(format!("failed to create output directory {d}").as_str());
        }
        let images: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|i|
                        if let Ok(it) = i {
                            if it.file_type().unwrap().is_file() {
                                Some(it.file_name().into_string().unwrap())
                            } else { None }
                        } else { None })
            .collect();

        eprintln!("file count: {}", images.len());
        assert!(images.len() > 0);

        let mut image_path = PathBuf::from(&dir);
        image_path.push(&images[0]);
        let image = wgpu::Texture::from_path(app, image_path).unwrap();

        // TODO: use hashset
        // TODO: manage
        Self {
            total_files: images.len(),
            current_image: (image, 0),
            new_name: String::new(),
            dir,
            images,
        }
    }

    pub fn next_image(&mut self, app: &App) {
        let max = self.images.len() - 1;
        self.current_image.1 += if self.current_image.1 >= max { 0 } else { 1 };
        self.reload_image(app);
    }

    pub fn prev_image(&mut self, app: &App) {
        self.current_image.1 -= if self.current_image.1 == 0 { 0 } else { 1 };
        self.reload_image(app);
    }

    pub fn seek_to_image(&mut self, app: &App, pos: usize) {
        let max = self.images.len() - 1;

        if self.current_image.1 != pos {
            if pos > max {
                eprint!("image index {pos} is out of bounds");
            }
            self.current_image.1 = if pos >= max { max } else { pos };

            self.reload_image(app)
        }
    }

    pub fn reload_image(&mut self, app: &App) {
        let mut image_path = PathBuf::from(&self.dir);

        image_path.push(&self.images[self.current_image.1]);

        println!("loaded image: {image_path:?}");
        if let Ok(img) = wgpu::Texture::from_path(app, image_path) {
            self.current_image.0 = img;
        } else {
            eprintln!("cannot open image");

            let p = app.assets_path().expect("missing assets folder")
                .join("img")
                .join(PLACEHOLDER_FILENAME);

            self.current_image.0 = wgpu::Texture::from_path(app, p)
                .expect("cannot open placeholder");

            // this causes a stack overflow if the last image is not readable
            // self.next_image(app);
        }
    }

    /// Path is prepended with no extra tokens so save can handle both separate and regular save
    pub fn move_current(&mut self, app: &App, category: &str, new_name: &str) {
        let f = &self.images[self.current_image.1];
        let f_full = self.dir.join(&f);

        let f_str: std::string::String =
            f.chars()
            .map(|c| if c == ' ' { '_' } else { c } ).collect();

        let output = self.dir.join(DIR_OUTPUT)
            .join(category)
            .join(
                format!("{}__{}",
                        new_name,
                        f_str,
                )
            );

        println!("moving file: {f_full:?} -> {output:?}");

        std::fs::copy(&f_full, &output).expect("failed to save file");

        std::fs::remove_file(&f_full)
            .expect("failed to  file");

        self.images.remove(self.current_image.1);
        self.reload_image(app);
        self.new_name.clear();
    }
}

/// Text suggestion engine
struct TextSuggester {
    categories: HashSet<String>,
    config_path: PathBuf,
    new_category_buffer: String,
    last_key: Option<egui::Key>,
}
impl TextSuggester {
    pub fn new(config_path: &PathBuf) -> Self {

        let config_path = config_path.join("cfg").join("categories.json");
        println!("cat config path: {config_path:?}");

        let categories =
            std::fs::read_to_string(&config_path)
            .expect("failed to read preference file");
        println!("categories: {categories}");

        Self {
            last_key: None,
            categories: serde_json::from_str(&categories)
                .expect("cannot parse categories preference file"),
            new_category_buffer: String::new(),
            config_path,
        }
    }


    pub fn get_suggestions(&mut self, prompt: &str) -> Vec<String> {
        get_results(&self.categories, prompt)
    }

    pub fn last_key_changed(&mut self, key: egui::Key) -> bool {
        if let Some(ref mut last_key) = self.last_key {
            if *last_key != key {
                *last_key = key;
                true
            } else {
                false
            }
        } else {
            self.last_key = Some(key);
            true
        }
    }

    pub fn add_category(&mut self) {
        println!("added new category: {}", self.new_category_buffer);
        self.categories.insert(self.new_category_buffer.clone());
        self.new_category_buffer.clear();

        let cfg_str = serde_json::to_string(&self.categories).unwrap();
        std::fs::write(&self.config_path, &cfg_str).expect("cannot write preferences");
    }
}
// outside because of caching method
#[cached(
    type = "SizedCache<String, Vec<String>>",
    create = "{ SizedCache::with_size(20) }",
    convert =  "{ prompt.to_string() }"
)]
fn get_results(categories: &HashSet<String>, prompt: &str) -> Vec<String> {
    use rust_fuzzy_search::fuzzy_compare;

    categories.iter()
        .filter_map(|cat| {
            let score = fuzzy_compare(cat, prompt);
            //println!("score: {score}");

            if score > 0.0 {
                Some(cat.clone())
            } else {
                None
            }
        }).collect()
}


/// Main user state for the application
struct Model {
    egui: Egui,
    text_suggest: TextSuggester,
    manager: ImageManager,
}
impl Model {
    pub fn new(app: &App, egui: Egui) -> Self {
        let folder =
            if let Ok(args) = Args::try_parse() {
                args.folder
            } else {
                println!("cannot parse cmd line args, using default path");
                let path_s = std::fs::read_to_string(
                    app.assets_path().unwrap().join("cfg").join("default_path")
                ).expect("cannot read default path");
                let path_s = path_s.trim();

                PathBuf::from(path_s)
            };

        Model {
            egui,
            text_suggest: TextSuggester::new(&app.assets_path().expect("cannot open project path")),
            manager: ImageManager::new(app, folder),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    folder: PathBuf,
}

/*--- Main application ---------------------------------------------------------------------------*/

fn main() {
    nannou::app(model)
        .update(update)
        .run();
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

    egui.set_elapsed_time(update.since_start);
    let egui_context = egui.begin_frame();

    let mut pos = manager.current_image.1 as f32;
    let max_img = (manager.images.len() - 1 ) as f32;

    // GUI layout
    //egui::Window::new("File Control").show(&egui_context, |ui| {
    egui::TopBottomPanel::bottom("File Control").show(&egui_context, |ui| {

        ui.label(format!("current image: {}", manager.images[manager.current_image.1]));
        ui.separator();

        ui.label("Controls");

        ui.add(egui::Slider::new(&mut pos, 0.0..=max_img).text("Skip to file"));
        manager.seek_to_image(app, pos as usize);

        let create_buttons = |col: &mut [egui::Ui] | {
            // let mut btn_ev: Vec<(Response, Box<dyn FnOnce(&mut ImageManager, Response)>)> =
            // Vec::new();

            {
                let c_ui = &mut col[0];
                c_ui.label("Prev");
                let btn = c_ui.add_enabled(manager.current_image.1 != 0 ,egui::Button::new(" ⏴ "));
                if btn.clicked() { manager.prev_image(app) }

                c_ui.label("Trash");
                let btn = c_ui.button("\u{1F5D1}"); // TODO: read btn state
                if btn.clicked() {
                    manager.move_current(app, DIR_TRASH, "trashed")
                }
            }

            {
                let c_ui = &mut col[1];
                c_ui.label("Next");
                let btn = c_ui.add_enabled(manager.current_image.1 != (manager.images.len()-1),
                                           egui::Button::new(" ⏵ "));
                if btn.clicked() { manager.next_image(app) }

                c_ui.label("Separate");
                let btn = c_ui.add(egui::Button::new(" \u{1F4E4} "));
                if btn.clicked() {
                    let name = manager.new_name.clone();
                    manager.move_current(app, DIR_OTHER, &name);
                }
            }

        };
        ui.columns(2, create_buttons);

        // percentage bar
        ui.separator();
        {
            ui.label("Remaining files");
            let p = 1.0 - ((manager.total_files as f32) / (manager.images.len() as f32)) ;
            ui.add(egui::ProgressBar::new(p)
                   .text(format!("{} / {} - {:.1} %",
                                 manager.total_files,
                                 manager.images.len(),
                                 p*100.0)));
        }

        // main input box
        ui.separator();
        ui.label("New file name:");
        let mut segments: Vec<String> = manager.new_name.split("--")
            .map(|s| s.to_string())
            .collect();

        let suggestions: Vec<String> = {
            // TODO: add constant for separator


            let suggestions = text_suggest.get_suggestions(&segments.last().unwrap_or(&" ".to_string()));

            let inputbox_r = ui.add(egui::TextEdit::singleline(&mut manager.new_name)
                            .code_editor()
                            .lock_focus(true)
                            //.cursor_at_end(true)
            );

            // detect confirmation
            if let Some(k) = ui.input().keys_down.iter().next() {
                if text_suggest.last_key_changed(*k) {
                    match k {
                        egui::Key::Enter => {
                            if inputbox_r.lost_focus() {
                                let name = manager.new_name.clone();
                                manager.move_current(app, DIR_OUTPUT, &name);
                                inputbox_r.request_focus();
                            }
                        },
                        egui::Key::Tab => {
                            // both replacement and destination are not empty
                            if let (Some(replacement), Some(dest)) =
                                (suggestions.first(), segments.last_mut())
                            {
                                *dest = replacement.to_string();
                                println!("completion: {replacement:?}");

                                // replace name with new string
                                manager.new_name = segments.iter()
                                    .fold(String::new(), |mut acc, part| {
                                        acc.push_str(part);
                                        acc
                                    });
                            }
                        }
                        _ => {},
                    }
                }
            }
            suggestions
        };

        let mut suggestions_iter = suggestions.iter();
        let first: String = suggestions_iter.next().map(|i| i.clone()).unwrap_or(" ".to_string());

        let _lab_h = ui.label(
            format!("Suggestions: {}",
                    suggestions_iter
                    .fold(first, |mut res, item| {
                        res.push_str(", ");
                        res.push_str(item);
                        res
                    })));

        // new categorie box
        ui.separator();
        ui.columns(2, |col| {
            if let Some(segment) = segments.last() {
                text_suggest.new_category_buffer
                    .replace(segment);
            }

            col[0].text_edit_singleline(&mut text_suggest.new_category_buffer);
            if col[1].button(" \u{002b} ").clicked() {
                    text_suggest.add_category();
            }
        });
    });
}
const PAD:f32 = 45.0;

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    frame.clear(BLACK);

    let win = app.window_rect();
    let canvas = Rect::from(win.clone()).top_left_of(win).pad_bottom(300.0);

    let [img_w, img_h] = model.manager.current_image.0.size();


    let mut dbg_msg: String;

    // scale image preserving proportions
    let wh: Vec2 =
    {
        let img_w_f = img_w as f32;
        let img_h_f = img_h as f32;
        let img_h_fit = img_h_f * (canvas.w() / img_w_f);
        let img_w_fit = img_w_f * (canvas.h() / img_h_f);

        if img_w > img_h && (img_h_fit+PAD) < canvas.h() {
            // fit to width
            dbg_msg = "wide".to_string();

            Vec2::new(canvas.w() - PAD, img_h_fit - PAD)
        } else {
            dbg_msg = "tall".to_string();

            // fit to height
            Vec2::new(img_w_fit - PAD, canvas.h() - PAD)
        }
    };

    let xy = Point2::new(canvas.x(), canvas.y());

    dbg_msg += &format!(" img wh: {wh:?}, \n canvas: .xy: {:?} .hw: {:?}",
                        canvas.xy(), canvas.wh()).to_string();

    // bg rect
    draw
        .rect()
        //.xy(canvas.wh())
        .wh(canvas.wh())
        .xy(canvas.xy())
        .color(DARKGREY);

    draw
        //.scissor(canvas)
        .texture(&model.manager.current_image.0)
        .wh(wh)
        .xy(xy);

    // debug text
        draw.text(&dbg_msg)
        .color(RED).font_size(20)
        .w(500.0)
        .xy(canvas.mid_bottom());

    // run queued drawing commands
    draw.to_frame(app, &frame).unwrap();

    // draw ui on top
    model.egui.draw_to_frame(&frame).unwrap();
}

/// Let egui handle things like keyboard and mouse input.
fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.egui.handle_raw_event(event);
}
