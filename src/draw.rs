use ggez::{Context, GameResult, GameError};
use ggez::graphics::Image;
use std::convert::TryInto;

pub trait ImageEx {
    fn get_region(&self, ctx: &mut Context, x0: &usize, y0: &usize, x1: &usize, y1: &usize) -> GameResult<Image>;
}

impl ImageEx for Image {
    fn get_region(&self, ctx: &mut Context, x0: &usize, y0: &usize, x1: &usize, y1: &usize) -> GameResult<Image> {
        // Validate positional arguments
        if x0 < &0usize || x1 < &0usize || y0 < &0usize || y1 < &0usize { 
            return Err(GameError::RenderError("One or more bounds are negative".to_string()));
        }
        if x0 > x1 { return Err(GameError::RenderError("First X bound greater than second bound".to_string())); }
        if y0 > y1 { return Err(GameError::RenderError("First Y bound greater than second bound".to_string())); }

        let height = Image::height(&self) as usize;
        let width = Image::width(&self) as usize;
        if x1 > &width || y1 > &height { return Err(GameError::RenderError("One or more bounds exceed image size".to_string())); }

        // Get selected region from image RGBA vector
        let rgba_vec: Vec<_> = Image::to_rgba8(&self, ctx).expect("Failed to build RGBA buffer for image!");
        let mut vec_region: Vec<u8> = Vec::new();
        for i in 0..rgba_vec.len() {
            let x_pos = i*4 % width;
            let y_pos = i*4 / width;
            if x_pos >= *x0 && x_pos < *x1 && y_pos >= *y0 && y_pos < *y1 { vec_region.push(rgba_vec[i]); }
        }
        let image_slice: Image = Image::from_rgba8(ctx, (x1 - x0).try_into().unwrap(), (y1 - y0).try_into().unwrap(), &vec_region).expect("Failed to create image from RGBA buffer!");
        Ok(image_slice)
    }
}