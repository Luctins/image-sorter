use std::{collections::{HashSet, HashMap}, fs::ReadDir, path::{Path, PathBuf}, sync::Arc};
use lazy_static::{lazy_static, __Deref};

use nannou::{image::DynamicImage, draw::primitive::Texture};
use nannou::image;

use crate::*;

/*--- Const --------------------------------------------------------------------------------------*/

pub mod dir {
    pub const TRASH: &'static str = "trash";
    pub const OUTPUT: &'static str = "output";
    pub const OTHER: &'static str = "separate";

    pub const DIRS: [&'static str; 3] = [TRASH, OUTPUT, OTHER];
}

/*--- Impl ---------------------------------------------------------------------------------------*/

lazy_static!{
    /// Supported file types
    static ref ALLOWED_FILE_TYPES: HashSet<String> = vec![ "png", "jpg", "jpeg", "webp"]
        .drain(..).map(|v| v.to_string()).collect();
    static ref PLACEHOLDER_BUF: &'static [u8] =
        std::include_bytes!("../assets/img/placeholder.bmp");
}

enum Action<'s> {
    /// in the end all actions are moves
    Move {
        source: &'s str,
        destination: &'s str,
    },
    //Undo(&Self),
    //Redo(&Self),
}

/// Image and file manager
pub struct ImageManager {
    pub filename_buffer: String,
    pub image_index: usize,
    pub images: Vec<String>,

    total_file_count: usize,
    dir: PathBuf,

    //action_history: Vec<Action<'s>>,

    image_current: DynamicImage,
    image_current_texture: Option<(Arc<wgpu::Texture>, usize)>,
}

impl ImageManager {
    pub const TAG_SEPARATOR: &'static str = "--";
    pub const PLACEHOLDER_FILENAME: &'static str = "missing-image-placeholder.png";

    pub fn new(images_path: PathBuf) -> Self {
        let dir = images_path.clone();
        println!("images path: {dir:?}");


        let images = Self::get_file_list(&dir);

        if images.len() == 0 {
            eprintln!("no supported files in current directory: {dir:?}");

            // TODO: show error message on the interface? (would require proper errors for this)

            std::process::exit(1);
        }

        for d in dir::DIRS {
            std::fs::create_dir_all(dir.join("output").join(d))
                .expect(format!("failed to create output directory {d}").as_str());
        }

        println!("file count: {}", images.len());

        let image_path = dir.join(&images[0]);
        println!("first image: {image_path:?}");
        let image_current = Self::open_image_or_default(image_path);
        //wgpu::Texture::from_path(app, image_path).unwrap();

        Self {
            //action_history: Vec::new(),
            filename_buffer: String::new(),
            image_current_texture: None,
            image_index: 0,
            total_file_count: images.len(),
            image_current,
            dir,
            images,
        }
    }

    pub fn get_total_files(&self) -> usize {
        self.total_file_count
    }

    pub fn get_images_len(&self) -> usize {
        self.images.len()
    }

    pub fn get_current_pos(&self) -> usize {
        self.image_index
    }

    pub fn get_current_path(&self) -> PathBuf {
        self.dir.join(self.get_current_filename())
    }

    pub fn get_current_filename<'s>(&'s self) -> &'s str {
        &self.images[self.image_index]
    }

    pub fn next_image(&mut self) {
        let max = self.images.len() - 1;
        self.image_index += if self.image_index >= max { 0 } else { 1 };
        self.reload_image();
    }

    pub fn prev_image(&mut self) {
        self.image_index -= if self.image_index == 0 { 0 } else { 1 };
        self.reload_image();
    }

    pub fn seek_to_image(&mut self, pos:usize) {
        let max = self.images.len() - 1;

        self.image_index = if pos >= max { max } else { pos };

        self.reload_image()
    }

    pub fn reload_image(&mut self) {
        let image_path = self.get_current_filename();

        self.image_current = Self::open_image_or_default(self.dir.join(image_path));
    }

    /// Path is prepended with no extra tokens so save can handle both separate and regular save
    ///
    /// Category is essentially the destination folder
    pub fn move_current(&mut self, category: &str, new_name: &str) {

        // TODO: implement act history
        // TODO: make category a enum? it's the destination folder

        let f = &self.images[self.image_index];
        let source_f = self.dir.join(f);

        // remove spaces from filename
        let f_str: std::string::String =
            f.chars().map(|c| if c == ' ' { '_' } else { c }).collect();

        let output_path = self
            .dir
            .join(dir::OUTPUT)
            .join(category)
            .join(format!("{}__{}", new_name.trim_end_matches("--"), f_str));

        println!("moving file: {source_f:?} -> {output_path:?}");

        std::fs::copy(&source_f, &output_path).expect("failed to save file");

        std::fs::remove_file(&output_path).expect("failed to  file");

        self.images.remove(self.image_index);
        self.reload_image();
        self.filename_buffer.clear();
    }


    /// get the Texture from the current image
    ///
    /// Panics if called before update texture
    // TODO: load default texture on new
    pub fn get_texture(&self) -> Arc<wgpu::Texture> {
        self.image_current_texture.clone().unwrap().0
    }

    pub fn update_texture(&mut self, app: &App) {
        if let Some((_, index)) = &mut self.image_current_texture {
            if *index != self.image_index {
                self.convert_img(app);
            }
        } else {
            self.convert_img(app);
        }
    }

    // -- private items
    fn open_image_or_default<P>(path: P) -> DynamicImage where P: AsRef<Path> + std::fmt::Debug {
        match image::open(path.as_ref()) {
            Ok(img) => {
                println!("opened image at: {path:?}");
                img
            }
            Err(_e) => {
                eprintln!("failed to open image at: {path:?}");
                image::load_from_memory_with_format(
                    PLACEHOLDER_BUF.as_ref(),
                    image::ImageFormat::Bmp
                ).unwrap()
            }
        }
    }


    fn get_file_list<P>(dir: P) -> Vec<String> where P: AsRef<Path> {
        // filter files in the directory that match certain criteria
        std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|i| {
                if let Ok(it) = i {
                    if let Ok(ft) = it.file_type() {
                        if ft.is_file() {
                            let filename = it.file_name().into_string().unwrap();
                            if let Some(extension) = filename.split('.').last() {
                                if ALLOWED_FILE_TYPES.contains(extension) {
                                    Some(filename)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
                })
            .collect()
    }

    fn convert_img(&mut self, app: &App) {
        self.image_current_texture =
            Some((
                Arc::new(wgpu::Texture::from_image(app, &self.image_current)),
                self.image_index
            ));
    }
}
