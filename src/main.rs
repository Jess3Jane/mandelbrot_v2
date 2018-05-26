extern crate image;
extern crate num_cpus;
extern crate spmc;
extern crate fractal;

use std::path::Path;
use std::thread;
use image::ImageBuffer;
use std::sync::{Arc, Mutex};
use fractal::{RenderingContext, ColorScheme};

fn render<F>(ctx: RenderingContext, cs: &ColorScheme, path: &Path, frac: F) where F: Fn(f64, f64, u64) -> u64 + Send + Sync + 'static{
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
    let mut handles = Vec::with_capacity(threads);
    let (tx, rx) = spmc::channel();
    let rc = Arc::new(frac);
    for id in 0..threads {
        let rx = rx.clone();
        let mut histogram = histograms[id].clone();
        let r = rc.clone();
        handles.push(thread::spawn(move || {
            let mut histogram = histogram.lock().unwrap();
            loop {
                match rx.recv().unwrap() {
                    Some((row, row_arc)) => 
                    {
                        let row_arc : Arc<Mutex<Vec<u64>>> = row_arc;
                        let mut row_iter = row_arc.lock().unwrap();
                        for (x0, y0, x_px) in row {

                            let iter = r(x0, y0, ctx.max_iter);

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
        tx.send(Some((row, row_iter))).unwrap();
    }

    for _ in 0..threads {
        tx.send(None).unwrap();
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
    image::ImageRgb8(img).save(path).unwrap();
}

fn main() {
    let mut cs = ColorScheme::new();
    cs.add_hex(0x000000, 0.0);
    cs.add_hex(0xbb2200, 0.8);
    cs.add_hex(0xff7700, 1.0);

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

    //let mut cs = ColorScheme::new();
    //cs.add_hex(0xffffff, 1.0-0.0);
    //cs.add_hex(0xffecb3, 1.0-0.2);
    //cs.add_hex(0xe85285, 1.0-0.45);
    //cs.add_hex(0x6a1b9a, 1.0-0.65);
    //cs.add_hex(0x000000, 1.0-1.0);

//    let ctx = RenderingContext { 
//        x: 0.0, y: 0.0, 
//        scale: 12.0, max_iter: 50, 
//        x_px: 1920, y_px: 1080,};

//    render(ctx, &cs, &Path::new("test.png"), |x0, y0, max_iter| {
//            let mut x = 0.0;
//            let mut y = 0.0;
//            let mut iter = 0;
//
//            while x*x + y*y < 4.0 && iter < max_iter {
//                let xtemp = x*x - y*y + x0;
//                let ytemp = 2.0*x*y + y0;
//
//                if x == xtemp && y == ytemp {
//                    iter = max_iter;
//                    break;
//                }
//
//                x = xtemp;
//                y = ytemp;
//                iter += 1;
//            }
//
//            iter
//        });

//    render(ctx, &cs, &Path::new("test.png"), |x0, y0, max_iter| {
//            let mut x = x0;
//            let mut y = y0;
//            let cx = 0.0;
//            let cy = 0.90;
//            let mut iter = 0;
//
//            while x*x + y*y < 4.0 && iter < max_iter {
//                let xtemp = x*x - y*y;
//                y = 2.0*x*y + cy;
//                x = xtemp + cx;
//                iter += 1;
//            }
//
//            iter
//        });


    let ctx = RenderingContext { 
        x: 0.0, y: 0.0, 
        scale: 12.0, max_iter: 50, 
        x_px: 1920, y_px: 1080,};

    render(ctx, &cs, &Path::new("test.png"), |x0, y0, max_iter| {
            let mut x = x0;
            let mut y = y0;
            let cx = 1.0;
            let cy = 1.0;
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
