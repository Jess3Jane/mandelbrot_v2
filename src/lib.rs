extern crate image;
extern crate spmc;
extern crate num_cpus;

mod context;
pub use self::context::{RenderingContext, ColorScheme};

mod util;
pub use self::util::render_image;
