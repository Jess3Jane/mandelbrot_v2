extern crate fractal;
extern crate image;
extern crate pbr;

use std::path::Path;
use fractal::{ColorScheme, RenderingContext};
use std::f64::consts::PI;
use std::rc::Rc;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use pbr::ProgressBar;
use image::ImageBuffer;

struct Frame {
    t: f64,
    image: Box<Vec<u64>>,
}

impl Frame {
    fn difference(&self, other: &Rc<Frame>) -> u64 {
        let mut total = 0;
        for i in 0..self.image.len() {
            total += (self.image[i] as i64 - other.image[i] as i64).abs() as u64;
        }
        total
    }
}

impl Ord for Frame {
    // This will fail when one of the t's is nan or infinity
    // which shouldn't really happen so whatever
    fn cmp(&self, other: &Frame) -> Ordering { 
        if self.t == other.t { Ordering::Equal }
        else if self.t > other.t { Ordering::Greater }
        else { Ordering::Less }
    }
}

impl PartialOrd for Frame {
    fn partial_cmp(&self, other: &Frame) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Eq for Frame { }

impl PartialEq for Frame { 
    fn eq(&self, other: &Frame) -> bool { self.t == other.t }
}

#[derive(Eq)]
struct Interval {
    a: Rc<Frame>,
    b: Rc<Frame>,
    difference: u64,
}

impl Interval {
    fn midpoint(&self) -> f64 {
        if self.b.t != 0.0 {
            (self.a.t + self.b.t)/2.0
        } else {
            (self.a.t + 1.0)/2.0
        }
    }

    fn subdivide(self, midpoint: Rc<Frame>) -> (Interval, Interval) {
        let a = Interval{ a: self.a.clone(), b: midpoint.clone(), difference: self.a.difference(&midpoint) };
        let b = Interval{ a: midpoint.clone(), b: self.b.clone(), difference: self.b.difference(&midpoint) };
        (a, b)
    }
}

impl Ord for Interval {
    fn cmp(&self, other: &Interval) -> Ordering { 
        self.difference.cmp(&other.difference)
    }
}

impl PartialOrd for Interval {
    fn partial_cmp(&self, other: &Interval) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl PartialEq for Interval {
    fn eq(&self, other: &Interval) -> bool { self.difference == other.difference }
}

fn render_frame<F>(ctx: &RenderingContext, frac: &F, t: f64) -> Frame where F: Fn(f64, f64, u64, f64) -> u64 + Send + Sync + 'static {
    let mut image = Vec::with_capacity(ctx.x_px as usize*ctx.y_px as usize);
    for _ in 0..ctx.x_px*ctx.y_px { image.push(0); }

    for (x0, y0, x_px, y_px) in ctx.enumerate_points() {
        let iter = frac(x0, y0, ctx.max_iter, t);
        image[x_px as usize + y_px as usize * ctx.x_px as usize] = iter;
    }

    let image = Box::new(image);
    Frame{image, t}
}

fn render_vfr<F>(ctx: RenderingContext, cs: ColorScheme, path: &'static Path, frame_count: u32, frac: F) where F: Fn(f64, f64, u64, f64) -> u64 + Send + Sync + 'static {
    let mut pb = ProgressBar::new(frame_count as u64);
    pb.format("[=> ]");
    pb.message("Rendering frames ");
    pb.add(0);

    let mut frames : Vec<Rc<Frame>> = Vec::with_capacity(4); 
    for i in 0..4 {
        frames.push(Rc::new(render_frame(&ctx, &frac, i as f64/4.0)));
        pb.inc();
    }

    let mut pq = BinaryHeap::new();
    pq.push(Interval{ a: frames[0].clone(), b: frames[1].clone(), difference: frames[0].difference(&frames[1]) });
    pq.push(Interval{ a: frames[1].clone(), b: frames[2].clone(), difference: frames[1].difference(&frames[2]) });
    pq.push(Interval{ a: frames[2].clone(), b: frames[3].clone(), difference: frames[2].difference(&frames[3]) });
    pq.push(Interval{ a: frames[3].clone(), b: frames[0].clone(), difference: frames[3].difference(&frames[0]) });

    while frames.len() < frame_count as usize {
        let interval = pq.pop().unwrap();
        let f = Rc::new(render_frame(&ctx, &frac, interval.midpoint()));
        frames.push(f.clone());
        let (a, b) = interval.subdivide(f);
        pq.push(a);
        pq.push(b);
        pb.inc();
    }

    pb.finish();

    frames.sort();

    for i in 0..frames.len() {
        eprintln!("{}, {}", i, frames[i as usize].t);
        let mut img = ImageBuffer::new(ctx.x_px, ctx.y_px);
        let image = &frames[i as usize].image;
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let iter = image[x as usize + y as usize*ctx.x_px as usize];
            if iter == ctx.max_iter {
                *pixel = image::Rgb([0, 0, 0]);
            } else {
                *pixel = cs.get_color(iter as f64/ctx.max_iter as f64);
            }
        }
        image::ImageRgb8(img).save(path.join(Path::new(&format!("frame{}.png", i)))).unwrap();
    }
}

fn main() {
    //let mut cs = ColorScheme::new();
    //cs.add_hex(0x000764, 0.0);
    //cs.add_hex(0x206bcb, 0.16);
    //cs.add_hex(0xedffff, 0.42);
    //cs.add_hex(0xffaa00, 0.6425);
    //cs.add_hex(0x000200, 0.8575);
    //cs.add_hex(0x000764, 1.0);

    //let mut cs = ColorScheme::new();
    //cs.add_hex(0x000000, 0.0);
    //cs.add_hex(0xffffff, 1.0);

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

    //let mut cs = ColorScheme::new();
    //cs.add_hex(0x00ffff, 0.0/2.0);
    //cs.add_hex(0xff00ff, 1.0/2.0);
    //cs.add_hex(0xffffff, 2.0/2.0);

    let ctx = RenderingContext { 
        x: 0.0, y: 0.0, 
        scale: 12.0, max_iter: 50, 
        x_px: 256, y_px: 256,};

    render_vfr(ctx, cs, Path::new("frames"), 3000, |x0, y0, max_iter, t| {
            let mut x = x0;
            let mut y = y0;
            let cx = (PI*t).sin();
            let cy = (PI*t).cos();
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
