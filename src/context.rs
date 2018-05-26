#[derive(Clone, Copy)]
pub struct RenderingContext {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
    pub max_iter: u64,
    pub x_px: u32,
    pub y_px: u32,
}

impl RenderingContext {
    pub fn enumerate_points(&self) -> ImageIterator {
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

    pub fn enumerate_rows(&self) -> RowIterator {
        let x_scale = self.scale;
        let y_scale = self.scale * ((self.y_px as f64)/(self.x_px as f64));
        let x_offset = self.x as f64 - x_scale/2.0;
        let y_offset = self.y as f64 - y_scale/2.0;
        RowIterator{
            x_scale, y_scale,
            x_offset, y_offset,
            cur_y: 0,
            x_px: self.x_px, y_px: self.y_px,
        }
    }
}

pub struct ImageIterator {
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

pub struct RowIterator {
    x_scale: f64,
    y_scale: f64,
    x_offset: f64,
    y_offset: f64,
    cur_y: u32,
    x_px: u32,
    y_px: u32,
}

impl Iterator for RowIterator {
    type Item = (RowPixelIterator, u32);
    fn next(&mut self) -> Option<(RowPixelIterator, u32)> {
        if self.cur_y >= self.y_px {
            self.cur_y = 0;
            return None
        }

        let ret = Some((RowPixelIterator{ 
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

pub struct RowPixelIterator {
    x_scale: f64,
    x_offset: f64,
    cur_x: u32,
    x_px: u32,
    y_offset: f64,
}

impl Iterator for RowPixelIterator {
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

use image::Rgb;

pub struct ColorSchemeColor {
    color: Rgb<u8>,
    position: f64,
}

impl ColorSchemeColor {
    fn from_hex(color: u32, position: f64) -> ColorSchemeColor {
        let r = (color >> 16) as u8;
        let g = (color >> 8) as u8;
        let b = color as u8;
        ColorSchemeColor {color: Rgb([r, g, b]), position}
    }
}

pub struct ColorScheme {
    colors: Vec<ColorSchemeColor>,
}

impl ColorScheme {
    pub fn new() -> ColorScheme {
        ColorScheme { colors: Vec::new() }
    }

    fn add_color(&mut self, color: ColorSchemeColor) {
        let mut i = 0;
        while i < self.colors.len() && self.colors[i].position < color.position { i += 1; }
        self.colors.insert(i, color);
    }

    pub fn add_hex(&mut self, color: u32, position: f64) {
        self.add_color(ColorSchemeColor::from_hex(color, position));
    }

    fn _lerp(a: u8, b: u8, f: f64) -> u8 {
        (a as f64 * (1.0 - f) + b as f64 * f) as u8
    }

    fn lerp(a: &Rgb<u8>, b: &Rgb<u8>, f: f64) -> Rgb<u8> {
        let r = ColorScheme::_lerp(a.data[0], b.data[0], f);
        let g = ColorScheme::_lerp(a.data[1], b.data[1], f);
        let b = ColorScheme::_lerp(a.data[2], b.data[2], f);
        Rgb([r,g,b])
    }

    pub fn get_color(&self, pos: f64) -> Rgb<u8> {
        let mut i = 0;
        while i < self.colors.len() && self.colors[i].position < pos { i += 1; }
        let a = &self.colors[i];
        let b = if i == 0 { &self.colors[0] } else { &self.colors[i-1] };
        ColorScheme::lerp(&a.color, &b.color, (pos - a.position) as f64/(b.position - a.position) as f64)
    }
}
