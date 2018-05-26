extern crate fractal;
extern crate num_cpus;
extern crate spmc;
extern crate image;

use std::path::Path;
use std::sync::Arc;
use std::thread;
use image::ImageBuffer;
use fractal::{ColorScheme, RenderingContext};

fn render_animation<F>(ctx: RenderingContext, cs: ColorScheme, path: &'static Path, frames: u32, frac: F) where F: Fn(f64, f64, u64, u32) -> u64 + Send + Sync + 'static{
    let mut handles = Vec::with_capacity(num_cpus::get());
    let (tx, rx) = spmc::channel();
    let frac = Arc::new(frac);
    let cs = Arc::new(cs);
    for _ in 0..num_cpus::get() {
        let rx = rx.clone();
        let frac = frac.clone();
        let cs = cs.clone();
        handles.push(thread::spawn(move || {
            loop {
                match rx.recv().unwrap() {
                    Some(frame) =>
                    {
                        let mut histogram = Vec::with_capacity(ctx.max_iter as usize);
                        for _ in 0..ctx.max_iter { histogram.push(0); }

                        let mut image = Vec::with_capacity(ctx.x_px as usize*ctx.y_px as usize);
                        for _ in 0..ctx.x_px*ctx.y_px { image.push(0); }

                        for (x0, y0, x_px, y_px) in ctx.enumerate_points() {
                            let iter = frac(x0, y0, ctx.max_iter, frame);

                            image[x_px as usize + y_px as usize*ctx.x_px as usize] = iter;
                            if iter != ctx.max_iter {
                                histogram[iter as usize] += 1;
                            }
                        }

                        let mut total = 0;
                        for i in 0..ctx.max_iter {
                            total += histogram[i as usize];
                            histogram[i as usize] = total;
                        }

                        let mut img = ImageBuffer::new(ctx.x_px, ctx.y_px);
                        for (x, y, pixel) in img.enumerate_pixels_mut() {
                            let mut iter = image[x as usize + y as usize*ctx.x_px as usize];
                            if iter == ctx.max_iter {
                                *pixel = image::Rgb([0, 0, 0]);
                            } else {
                                let pos = histogram[iter as usize] as f64 / (ctx.x_px*ctx.y_px) as f64;
                                *pixel = cs.get_color(pos);
                            }
                        }

                        image::ImageRgb8(img).save(path.join(Path::new(&format!("frame{}.png",frame)))).unwrap();
                        println!("Finished frame {}", frame);
                    },
                    None => break,
                }
            }
        }));

    }
    for i in 0..frames {
        tx.send(Some(i)).unwrap();
    }

    for _ in 0..handles.len(){
        tx.send(None).unwrap();
    }

    for thread in handles{
        thread.join().unwrap();
    }
}

fn main() {
    //let mut cs = ColorScheme::new();
    //cs.add_hex(0x000000, 0.0);
    //cs.add_hex(0xbb2200, 0.8);
    //cs.add_hex(0xff7700, 1.0);

    //let mut cs = ColorScheme::new();
	//cs.add_hex(0xfff7f3, 9.0/9.0);
	//cs.add_hex(0xfde0dd, 8.0/9.0);
	//cs.add_hex(0xfcc5c0, 7.0/9.0);
	//cs.add_hex(0xfa9fb5, 6.0/9.0);
	//cs.add_hex(0xf768a1, 5.0/9.0);
	//cs.add_hex(0xdd3497, 4.0/9.0);
	//cs.add_hex(0xae017e, 3.0/9.0);
	//cs.add_hex(0x7a0177, 2.0/9.0);
	//cs.add_hex(0x49006a, 1.0/9.0);
	//cs.add_hex(0x000000, 0.0/9.0);

    let mut cs = ColorScheme::new();
    cs.add_hex(0xffffff, 1.0-0.0);
    cs.add_hex(0xffecb3, 1.0-0.2);
    cs.add_hex(0xe85285, 1.0-0.45);
    cs.add_hex(0x6a1b9a, 1.0-0.65);
    cs.add_hex(0x000000, 1.0-1.0);

    let ctx = RenderingContext { 
        x: 0.0, y: 0.0, 
        scale: 4.0, max_iter: 50, 
        x_px: 1920, y_px: 1080,};

    render_animation(ctx, cs, Path::new("frames"), 30, |x0, y0, max_iter, frame| {
            let mut x = x0;
            let mut y = y0;
            let cx = 1.0;
            let cy = 1.0*frame as f64/30.0;
            let mut iter = 0;
    
            while y.abs() < 50.0 && iter < max_iter {
                let xtemp = x.sin()*y.cosh();
                let ytemp = x.cos()*y.sinh();
                x = cx*xtemp - cy*ytemp;
                y = cx*ytemp + cy*xtemp;
                iter += 1;
            }
    
            iter
        });
}
