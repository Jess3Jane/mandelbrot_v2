use std::path::Path;
use std::thread;
use std::sync::{Arc, Mutex};
use super::{RenderingContext, ColorScheme};
use num_cpus;
use spmc;
use image;
use pbr::ProgressBar;
use image::ImageBuffer;

///    render_image(ctx, &cs, &Path::new("test.png"), |x0, y0, max_iter| {
///            let mut x = 0.0;
///            let mut y = 0.0;
///            let mut iter = 0;
///
///            while x*x + y*y < 4.0 && iter < max_iter {
///                let xtemp = x*x - y*y + x0;
///                let ytemp = 2.0*x*y + y0;
///
///                if x == xtemp && y == ytemp {
///                    iter = max_iter;
///                    break;
///                }
///
///                x = xtemp;
///                y = ytemp;
///                iter += 1;
///            }
///
///            iter
///        });
///
///    render_image(ctx, &cs, &Path::new("test.png"), |x0, y0, max_iter| {
///            let mut x = x0;
///            let mut y = y0;
///            let cx = 0.0;
///            let cy = 0.90;
///            let mut iter = 0;
///
///            while x*x + y*y < 4.0 && iter < max_iter {
///                let xtemp = x*x - y*y;
///                y = 2.0*x*y + cy;
///                x = xtemp + cx;
///                iter += 1;
///            }
///
///            iter
///        });
///
///    render_image(ctx, &cs, &Path::new("test.png"), |x0, y0, max_iter| {
///            let mut x = x0;
///            let mut y = y0;
///            let cx = 1.0;
///            let cy = 1.0;
///            let mut iter = 0;
///    
///            while y.abs() < 50.0 && iter < max_iter {
///                let xtemp = x.sin()*y.cosh();
///                let ytemp = x.cos()*y.sinh();
///                x = cx*xtemp - cy*ytemp;
///                y = cx*ytemp + cy*xtemp;
///                iter += 1;
///            }
///    
///            iter
///        });

pub fn render_image<F>(ctx: RenderingContext, cs: &ColorScheme, path: &Path, frac: F) where F: Fn(f64, f64, u64) -> u64 + Send + Sync + 'static{
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

    let mut pb = ProgressBar::new(ctx.y_px as u64);
    pb.format("[=> ]");
    pb.message("Rendering Rows ");
    pb.add(0);
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
    pb.finish();

    let mut histogram : Vec<u64> = Vec::with_capacity(ctx.max_iter as usize);
    for _ in 0..ctx.max_iter { histogram.push(0); }

    let mut total = 0;
    for i in 0..ctx.max_iter {
        for j in 0..threads {
            total += histograms[j as usize].lock().unwrap()[i as usize];
        }
        histogram[i as usize] = total;
    }

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

    image::ImageRgb8(img).save(path).unwrap();
}

pub fn render_animation<F>(ctx: RenderingContext, cs: ColorScheme, path: &'static Path, frames: u32, frac: F) where F: Fn(f64, f64, u64, u32) -> u64 + Send + Sync + 'static{
    let mut handles = Vec::with_capacity(num_cpus::get());
    let (tx, rx) = spmc::channel();
    let frac = Arc::new(frac);
    let cs = Arc::new(cs);

    let mut pb = ProgressBar::new(frames as u64);
    pb.format("[=> ]");
    pb.message("Allocating images ");
    let mut images = Vec::with_capacity(frames as usize);
    for _ in 0..frames {
        let mut image = Vec::with_capacity(ctx.x_px as usize*ctx.y_px as usize);
        for _ in 0..ctx.x_px*ctx.y_px { image.push(0); }
        images.push(Arc::new(Mutex::new(image)));
        pb.inc();
    }
    pb.finish();
    
    let mut histograms : Vec<Arc<Mutex<Vec<u64>>>> = Vec::with_capacity(num_cpus::get());
    for _ in 0..num_cpus::get() {
        let mut histogram = Vec::with_capacity(ctx.max_iter as usize);
        for _ in 0..ctx.max_iter { histogram.push(0); }
        histograms.push(Arc::new(Mutex::new(histogram)));
    }


    let mut pb = ProgressBar::new(frames as u64);
    pb.format("[=> ]");
    pb.message("Rendering frames ");
    pb.add(0);
    let pb = Arc::new(Mutex::new(pb));
    for i in 0..num_cpus::get() {
        let rx = rx.clone();
        let frac = frac.clone();
        let histogram = histograms[i].clone();
        let pb = pb.clone();
        handles.push(thread::spawn(move || {
            let mut histogram = histogram.lock().unwrap();
            loop {
                match rx.recv().unwrap() {
                    Some((dest, frame)) =>
                    {
                        let dest : Arc<Mutex<Vec<u64>>> = dest;
                        let mut image = dest.lock().unwrap();

                        for (x0, y0, x_px, y_px) in ctx.enumerate_points() {
                            let iter = frac(x0, y0, ctx.max_iter, frame);
                            image[x_px as usize + y_px as usize*ctx.x_px as usize] = iter;
                        }

                        for (_, _, x_px, y_px) in ctx.enumerate_points() {
                            let iter = image[x_px as usize + y_px as usize*ctx.x_px as usize];
                            if iter == ctx.max_iter { continue; }
                            let mut conv = 0;
                            for a in 0..3 {
                                for b in 0..3 {
                                    let a = a as i32 - 1;
                                    let b = b as i32 - 1;
                                    if a == 0 && b == 0 { continue; }
                                    if x_px as i32 + a < 0 || x_px as i32 + a >= ctx.x_px as i32 { continue; }
                                    if y_px as i32 + b < 0 || y_px as i32 + b >= ctx.y_px as i32 { continue; }
                                    let index = (x_px as i32 + a + (y_px as i32 + b)*ctx.x_px as i32) as usize;
                                    conv += (iter as i64 - image[index] as i64).abs();
                                }
                            }
                            histogram[iter as usize] += conv as u64
                        }

                        pb.lock().unwrap().inc();
                    },
                    None => break,
                }
            }
        }));

    }

    for i in 0..frames {
        tx.send(Some((images[i as usize].clone(), i))).unwrap();
    }

    for _ in 0..handles.len(){
        tx.send(None).unwrap();
    }

    for thread in handles {
        thread.join().unwrap();
    }

    pb.lock().unwrap().finish_print("done");

    let mut histogram : Vec<u64> = Vec::with_capacity(ctx.max_iter as usize);
    for _ in 0..ctx.max_iter { histogram.push(0); }

    let mut total : u64 = 0;
    for i in 0..ctx.max_iter {
        for j in 0..histograms.len() {
            total += histograms[j].lock().unwrap()[i as usize];
        }
        histogram[i as usize] = total;
    }

    let mut handles = Vec::with_capacity(num_cpus::get());
    let (tx, rx) = spmc::channel();

    let mut pb = ProgressBar::new(frames as u64);
    pb.format("[=> ]");
    pb.message("Writing images ");
    pb.add(0);
    let pb = Arc::new(Mutex::new(pb));
    let histogram = Arc::new(histogram);
    for _ in 0..num_cpus::get() {
        let rx = rx.clone();
        let histogram = histogram.clone();
        let cs = cs.clone();
        let pb = pb.clone();
        handles.push(thread::spawn(move || {
            loop {
                match rx.recv().unwrap() {
                    Some((img, frame)) => {
                        let img : Arc<Mutex<Vec<u64>>> = img;
                        let image = img.lock().unwrap();
                        let mut img = image::ImageBuffer::new(ctx.x_px, ctx.y_px);
                        for (x, y, pixel) in img.enumerate_pixels_mut() {
                            let mut iter = image[x as usize + y as usize*ctx.x_px as usize];
                            if iter == ctx.max_iter {
                                *pixel = image::Rgb([0, 0, 0]);
                            } else {
                                let pos = histogram[iter as usize] as f64 / total as f64;
                                *pixel = cs.get_color(pos);
                            }
                        }
                        image::ImageRgb8(img).save(path.join(Path::new(&format!("frame{}.png", frame)))).unwrap();
                        pb.lock().unwrap().inc();
                    },
                    None => break,
                }
            }
        }));
    }

    for i in 0..frames {
        tx.send(Some((images[i as usize].clone(), i))).unwrap();
    }

    for _ in 0..handles.len() {
        tx.send(None).unwrap();
    }

    for thread in handles {
        thread.join().unwrap();
    }
    pb.lock().unwrap().finish();
}

