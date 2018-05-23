extern crate image;
extern crate num_cpus;
extern crate spmc;

use std::iter::Iterator;
use std::path::Path;
use std::thread;
use image::ImageBuffer;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
struct RenderingContext {
    x: f64,
    y: f64,
    scale: f64,
    max_iter: u64,
    x_px: u32,
    y_px: u32,
}

impl RenderingContext {
    fn enumerate_points(&self) -> ImageIterator {
        let x_scale = self.scale;
        let y_scale = self.scale * ((self.y_px as f64)/(self.x_px as f64));
        let x_offset = self.x as f64 - x_scale/2.0;
        let y_offset = self.y as f64 - y_scale/2.0;
        ImageIterator{
            x_scale, y_scale,
            x_offset, y_offset,
            cur_x: 0, cur_y: 0,
            x_px: self.x_px, y_px: self.y_px,
        }
    }

    fn enumerate_rows(&self) -> ImageRowIterator {
        let x_scale = self.scale;
        let y_scale = self.scale * ((self.y_px as f64)/(self.x_px as f64));
        let x_offset = self.x as f64 - x_scale/2.0;
        let y_offset = self.y as f64 - y_scale/2.0;
        ImageRowIterator{
            x_scale, y_scale,
            x_offset, y_offset,
            cur_y: 0,
            x_px: self.x_px, y_px: self.y_px,
        }
    }
}

struct ImageIterator {
    x_scale: f64,
    y_scale: f64,
    x_offset: f64,
    y_offset: f64,
    cur_x: u32,
    cur_y: u32,
    x_px: u32,
    y_px: u32,
}

impl Iterator for ImageIterator {
    type Item = (f64, f64, u32, u32);
    fn next(&mut self) -> Option<(f64, f64, u32, u32)> {
        if self.cur_x >= self.x_px { 
            self.cur_x = 0;
            self.cur_y += 1;
            if self.cur_y >= self.y_px {
                self.cur_y = 0;
                return None
            }
        }

        let ret = Some((self.x_scale*(self.cur_x as f64/self.x_px as f64) + self.x_offset,
              self.y_scale*(self.cur_y as f64/self.y_px as f64) + self.y_offset,
              self.cur_x, self.cur_y));
        self.cur_x += 1;
        ret
    }
}

struct ImageRowIterator {
    x_scale: f64,
    y_scale: f64,
    x_offset: f64,
    y_offset: f64,
    cur_y: u32,
    x_px: u32,
    y_px: u32,
}

impl Iterator for ImageRowIterator {
    type Item = (ImageRowPixelIterator, u32);
    fn next(&mut self) -> Option<(ImageRowPixelIterator, u32)> {
        if self.cur_y >= self.y_px {
            self.cur_y = 0;
            return None
        }

        let ret = Some((ImageRowPixelIterator{ 
            x_scale: self.x_scale,
            x_offset: self.x_offset,
            cur_x: 0,
            x_px: self.x_px,
            y_offset: self.y_scale*(self.cur_y as f64/self.y_px as f64) + self.y_offset,
        }, self.cur_y));
        self.cur_y += 1;
        ret
    }
}

struct ImageRowPixelIterator {
    x_scale: f64,
    x_offset: f64,
    cur_x: u32,
    x_px: u32,
    y_offset: f64,
}

impl Iterator for ImageRowPixelIterator {
    type Item = (f64, f64, u32);
    fn next(&mut self) -> Option<(f64, f64, u32)> {
        if self.cur_x >= self.x_px {
            self.cur_x = 0;
            return None
        }

        let ret = Some((
                self.x_scale*(self.cur_x as f64/self.x_px as f64) + self.x_offset,
                self.y_offset, self.cur_x
            ));
        self.cur_x += 1;
        ret
    }
}

struct ColorSchemeColor {
    color: image::Rgb<u8>,
    position: f64,
}

impl ColorSchemeColor {
    fn from_hex(color: u32, position: f64) -> ColorSchemeColor {
        let r = (color >> 16) as u8;
        let g = (color >> 8) as u8;
        let b = color as u8;
        ColorSchemeColor {color: image::Rgb([r, g, b]), position}
    }
}

struct ColorScheme {
    colors: Vec<ColorSchemeColor>,
}

impl ColorScheme {
    fn new() -> ColorScheme {
        ColorScheme { colors: Vec::new() }
    }

    fn add_color(&mut self, color: ColorSchemeColor) {
        let mut i = 0;
        while i < self.colors.len() && self.colors[i].position < color.position { i += 1; }
        self.colors.insert(i, color);
    }

    fn _lerp(a: u8, b: u8, f: f64) -> u8 {
        (a as f64 * (1.0 - f) + b as f64 * f) as u8
    }

    fn lerp(a: &image::Rgb<u8>, b: &image::Rgb<u8>, f: f64) -> image::Rgb<u8> {
        let r = ColorScheme::_lerp(a.data[0], b.data[0], f);
        let g = ColorScheme::_lerp(a.data[1], b.data[1], f);
        let b = ColorScheme::_lerp(a.data[2], b.data[2], f);
        image::Rgb([r,g,b])
    }

    fn get_color(&self, pos: f64) -> image::Rgb<u8> {
        let mut i = 0;
        while i < self.colors.len() && self.colors[i].position < pos { i += 1; }
        let a = &self.colors[i];
        let b = if i == 0 { &self.colors[0] } else { &self.colors[i-1] };
        ColorScheme::lerp(&a.color, &b.color, (pos - a.position) as f64/(b.position - a.position) as f64)
    }
}

fn main() {
    let mut cs = ColorScheme::new();
    cs.add_color(ColorSchemeColor::from_hex(0x000000, 0.0));
    cs.add_color(ColorSchemeColor::from_hex(0xbb2200, 0.8));
    cs.add_color(ColorSchemeColor::from_hex(0xff7700, 1.0));

    let ctx = RenderingContext { 
        x: -1.7590170270659, y: 0.01916067191295, 
        scale: 1.1e-12, max_iter: 20000, 
        x_px: 1920, y_px: 1080,};

    let mut iters : Vec<Arc<Mutex<Vec<u64>>>> = Vec::with_capacity(ctx.y_px as usize);
    for _ in 0..ctx.y_px {
        let mut row : Vec<u64> = Vec::with_capacity(ctx.x_px as usize);
        for _ in 0..ctx.x_px {
            row.push(0);
        }
        iters.push(Arc::new(Mutex::new(row)));
    }

    let threads = num_cpus::get();
    let mut histograms : Vec<Arc<Mutex<Vec<u64>>>> = Vec::with_capacity(threads);
    for _ in 0..threads {
        let mut hist = Vec::with_capacity(ctx.max_iter as usize);
        for _ in 0..ctx.max_iter { hist.push(0); }
        histograms.push(Arc::new(Mutex::new(hist)));
    }

    println!("Calculating fractal");
    let mut handles = Vec::new();
    let (tx, rx) = spmc::channel();
    for id in 0..threads {
        let rx = rx.clone();
        let mut histogram = histograms[id].clone();
        handles.push(thread::spawn(move || {
            let mut histogram = histogram.lock().unwrap();
            loop {
                match rx.recv().unwrap() {
                    Some((row, row_arc)) => 
                    {
                        let row_arc : Arc<Mutex<Vec<u64>>> = row_arc;
                        let mut row_iter = row_arc.lock().unwrap();
                        for (x0, y0, x_px) in row {
                            let mut x = 0.0;
                            let mut y = 0.0;
                            let mut iter = 0;

                            while x*x + y*y < 4.0 && iter < ctx.max_iter {
                                let xtemp = x*x - y*y + x0;
                                let ytemp = 2.0*x*y + y0;

                                if x == xtemp && y == ytemp {
                                    iter = ctx.max_iter;
                                    break;
                                }

                                x = xtemp;
                                y = ytemp;
                                iter += 1;
                            }

                            row_iter[x_px as usize] = iter;
                            if iter != ctx.max_iter { 
                                histogram[iter as usize] += 1;
                            }
                        }
                    },
                    None => break,
                }
            }
        }));
    }

    for (row, y_px) in ctx.enumerate_rows() {
        let mut row_iter =  iters[y_px as usize].clone();
        tx.send(Some((row, row_iter)));
    }

    for _ in 0..threads {
        tx.send(None);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Calculating color map");
    let mut histogram : Vec<u64> = Vec::with_capacity(ctx.max_iter as usize);
    for _ in 0..ctx.max_iter { histogram.push(0); }

    let mut total = 0;
    for i in 0..ctx.max_iter {
        for j in 0..threads {
            total += histograms[j as usize].lock().unwrap()[i as usize];
        }
        histogram[i as usize] = total;
    }

    println!("Coloring image");
    let mut img = ImageBuffer::new(ctx.x_px, ctx.y_px);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let mut iter = iters[y as usize].lock().unwrap()[x as usize];
        if iter == ctx.max_iter {
            *pixel = image::Rgb([0, 0, 0]);
        } else {
            let pos = histogram[iter as usize] as f64 / (ctx.x_px*ctx.y_px) as f64;
            *pixel = cs.get_color(pos);
        }
    }

    println!("Saving");
    image::ImageRgb8(img).save(Path::new("test.png")).unwrap();
}
