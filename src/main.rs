use std::{path::PathBuf, fs::ReadDir};

use nannou::{prelude::*, image::DynamicImage};

use clap::Parser;

struct State {
    dir: PathBuf,
    images: Vec<PathBuf>,
    current_image: (wgpu::Texture, usize),
}
impl State {
    fn new(app: &App) -> Self {
        let dir = PathBuf::from("/home/luctins/tmp/sort/");

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

        State {
            dir,
            images,
            current_image: (image, 0)
        }
    }

    fn next_image(&mut self, app: &App) {
        let mut image_path = PathBuf::from(&self.dir);

        image_path.push(&self.images[0]);

        self.current_image.0 = wgpu::Texture::from_path(app, image_path).unwrap();
        self.current_image.1 += 1;
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    folder: PathBuf,
}

fn main() {
    nannou::app(model)
        .event(event)
        .simple_window(view)
        .run();
}

fn model(app: &App) -> State {
    State::new(app)
}

fn event(app: &App, state: &mut State, ev: Event) {
    use nannou::event::Event;
    use nannou::winit::event::DeviceEvent;

    match ev {
        Event::DeviceEvent(_id, DeviceEvent::Button { button, state: mouse_state}) => {
            state.next_image(app)
        }
        _ => {},
    }
}

fn view(app: &App, state: &State, frame: Frame) {
    let draw = app.draw();

    let win = app.window_rect();
    let canvas = Rect::from(win.clone()).top_left_of(win);
    // TODO: create a 'canvas' for separating the UI from the image based on ratios using rect

    let [img_h, img_w] = state.current_image.0.size();

    // scale image preserving proportions
    let (xy, wh): (Point2, Vec2) =
        {
            let img_w_fit = (img_w as f32) * (canvas.h() / (img_h as f32));

            if img_w > img_h || canvas.w() < img_w_fit {
                let img_h_fit = (img_h as f32) * (canvas.w() / (img_w as f32));
                // image is wide, fit to width
                (
                    Point2::new(0.0, 0.0),
                    Vec2::new(canvas.w(), img_h_fit)
                )
            } else {
                // image is tall, fit to height
                (
                    Point2::new(0.0, 0.0),
                    Vec2::new(img_w_fit, canvas.h())
                )
            }
        };
    //println!("wh: {wh:?}, canvas: {:?}", canvas.wh());

    draw.texture(&state.current_image.0)
        .xy(xy)
        .wh(wh);

    draw.to_frame(app, &frame).unwrap()
}
