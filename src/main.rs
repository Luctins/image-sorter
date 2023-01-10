#![allow(unused_imports)]
use std::{path::PathBuf, fs::ReadDir, sync::Arc, collections::HashSet};

use nannou::prelude::*;
use nannou_egui::{self, egui::{self, TextEdit, TextBuffer, Response}, Egui};

use cached::{proc_macro::cached, SizedCache};

use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;

use clap::Parser;

const PLACEHOLDER_FILENAME: &str = "missing-image-placeholder.png";

struct ImageManager {
    total_files: usize,
    dir: PathBuf,
    images: Vec<PathBuf>,
    new_name: String,
    current_image: (wgpu::Texture, usize),

    // /// Image placeholder
    //placeholder: ImageBuffer<Rgb<u8>, Vec<u8>>,
}
impl ImageManager {
    fn new(app: &App) -> Self {
        let dir = PathBuf::from("/home/luctins/tmp/meme");

        let images: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|i|
                        if let Ok(it) = i {
                            if it.file_type().unwrap().is_file() {
                                Some(PathBuf::from(it.file_name()))
                            } else { None }
                        } else { None })
            .collect();

        eprintln!("file count: {}", images.len());

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
}


// structstruck::strike!{
//     #[strikethrough[derive(Deserialize, Serialize)]]
//       struct TextSuggester {
//           categories: HashSet<String>,
//       }
// }

#[derive(Deserialize, Serialize)]
struct TextSuggester {
    categories: HashSet<String>,
}


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

impl TextSuggester {
    pub fn new() -> Self {
        // TODO: load hashset from file

        Self {
            categories:  vec![
                "programming",
                "warframe",
                "linux",
            ].into_iter()
                .map(|v| v.to_string())
                .collect()
        }
    }


    pub fn get_suggestions(&mut self, prompt: &str) -> Vec<String> {
        get_results(&self.categories, prompt)
    }
}

/// main user state for the application
struct Model {
    egui: Egui,
    text_suggest: TextSuggester,
    manager: ImageManager,
}

impl Model {
    pub fn new(app: &App, egui: Egui) -> Self {
        Model {
            egui,
            text_suggest: TextSuggester::new(),
            manager: ImageManager::new(app),
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
    egui::Window::new("File Control").show(&egui_context, |ui| {
        ui.label("Controls");

        ui.add(egui::Slider::new(&mut pos, 0.0..=max_img).text("Skip to file"));
        manager.seek_to_image(app, pos as usize);

        let create_buttons = |col: &mut [egui::Ui] | {
            // let mut btn_ev: Vec<(Response, Box<dyn FnOnce(&mut ImageManager, Response)>)> =
            // Vec::new();

            {
                let c_ui = &mut col[0];
                c_ui.label("Prev");
                let btn = c_ui.add(egui::Button::new(" ⏴ "));
                if btn.clicked() { manager.prev_image(app) }

                c_ui.label("Trash");
                let btn = c_ui.button("\u{1F5D1}"); // TODO: read btn state
                if btn.clicked() {
                    todo!("implement trash")
                }
            }

            {
                let c_ui = &mut col[1];
                c_ui.label("Next");
                let btn = c_ui.add(egui::Button::new(" ⏵ "));
                if btn.clicked() { manager.next_image(app) }

                c_ui.label("Separate");
                let btn = c_ui.add(egui::Button::new(" \u{1F4E4} "));
                if btn.clicked() { todo!("implement separating images with current category") }
            }

        };

        ui.columns(2, create_buttons);

        // TODO: create buttons for separating and and other options

        ui.separator();
        {
            ui.label("Remaining files");
            let p = (manager.total_files as f32) / (manager.images.len() as f32);
            ui.add(egui::ProgressBar::new(p)
                   .text(format!("{}", manager.images.len())));
        }

        ui.separator();

        ui.label("New file name:");
        let suggestions: Vec<String> = {
            let r = ui.add(egui::TextEdit::singleline(&mut manager.new_name)
                    .code_editor());

            // NOTE: i could use r.changed() in the future to improve performance

            text_suggest.get_suggestions(&manager.new_name)
        };

        let mut sug_iter = suggestions.iter();
        let first: String = sug_iter.next().map(|i| i.clone()).unwrap_or(" ".to_string());

        ui.label(format!("Suggestions: {} (TAB to cycle)",
                         sug_iter
                         .fold(first,
                               |mut res, item| {
                                   res.push_str(", ");
                                   res.push_str(item);
                                   res
                         })
        ));
    });


}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    frame.clear(BLACK);

    let win = app.window_rect();
    let canvas = Rect::from(win.clone()).top_left_of(win);
    // TODO: create a 'canvas' for separating the UI from the image based on ratios using rect

    let [img_w, img_h] = model.manager.current_image.0.size();

    // scale image preserving proportions
    let (xy, wh): (Point2, Vec2) =
        {
            let img_w_fit = (img_w as f32) * (canvas.h() / (img_h as f32));

            if img_w > img_h || canvas.w() < img_w_fit {
                let img_h_fit = (img_h as f32) * (canvas.w() / (img_w as f32));
                //println!("wide");

                // image is wide, fit to width
                (
                    Point2::new(0.0, 0.0),
                    Vec2::new(canvas.w(), img_h_fit)
                )
            } else {
                //println!("tall");

                // image is tall, fit to height
                (
                    Point2::new(0.0, 0.0),
                    Vec2::new(img_w_fit, canvas.h())
                )
            }
        };
    //println!("wh: {wh:?}, canvas: {:?}", canvas.wh());

    draw.texture(&model.manager.current_image.0)
        .xy(xy)
        .wh(wh);

    draw.to_frame(app, &frame).unwrap();

    model.egui.draw_to_frame(&frame).unwrap();
}

/// Let egui handle things like keyboard and mouse input.
fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.egui.handle_raw_event(event);
}
