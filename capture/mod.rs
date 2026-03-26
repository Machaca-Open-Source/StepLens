use anyhow::Result;
use std::path::PathBuf;
use image::{ImageFormat, DynamicImage, imageops, load_from_memory};

#[cfg(target_os = "linux")]
pub struct ScreenshotCapturer;

#[cfg(target_os = "linux")]
impl ScreenshotCapturer {
    // Capture a small region around a point (fast, only captures what's needed)
    pub fn capture_region(output_dir: &PathBuf, step_id: &str, x: Option<i32>, y: Option<i32>, region_size: u32) -> Result<PathBuf> {
        std::fs::create_dir_all(output_dir)?;
        use screenshots::Screen;
        
        let screens = Screen::all()?;
        if screens.is_empty() {
            return Err(anyhow::anyhow!("No screens found"));
        }

        let screen = screens.first().unwrap();
        let image = screen.capture().map_err(|e| anyhow::anyhow!("Capture failed: {}", e))?;
        
        let width = image.width();
        let height = image.height();
        
        // If we have click coordinates, crop to region around it
        let (crop_x, crop_y, crop_w, crop_h) = if let (Some(click_x), Some(click_y)) = (x, y) {
            // Capture region around click point
            let half_w = (region_size / 2) as i32;
            let half_h = (region_size / 2) as i32;
            
            let x_start = (click_x - half_w).max(0).min(width as i32 - region_size as i32).max(0) as u32;
            let y_start = (click_y - half_h).max(0).min(height as i32 - region_size as i32).max(0) as u32;
            let w = region_size.min(width.saturating_sub(x_start));
            let h = region_size.min(height.saturating_sub(y_start));
            
            (x_start, y_start, w, h)
        } else {
            // No coordinates - capture center region (smaller than full screen for speed)
            let center_x = (width / 2).saturating_sub(region_size / 2);
            let center_y = (height / 2).saturating_sub(region_size / 2);
            let w = region_size.min(width.saturating_sub(center_x));
            let h = region_size.min(height.saturating_sub(center_y));
            (center_x, center_y, w, h)
        };
        
        // Get PNG data with fastest compression
        use screenshots::Compression as PngCompression;
        let png_data = image.to_png(Some(PngCompression::Fast))
            .map_err(|e| anyhow::anyhow!("PNG encode failed: {}", e))?;
        
        // Load image
        let img = load_from_memory(&png_data)
            .map_err(|e| anyhow::anyhow!("Image load failed: {}", e))?;
        
        // Crop to region if we have coordinates
        let cropped = if let (Some(_), Some(_)) = (x, y) {
            // Crop the image to the region
            let cropped_view = imageops::crop_imm(&img, crop_x, crop_y, crop_w, crop_h);
            cropped_view.to_image()
        } else {
            // No coordinates - just resize smaller
            let new_w = width.min(640);
            let new_h = (height as f32 * (new_w as f32 / width as f32)) as u32;
            imageops::resize(&img, new_w, new_h, imageops::FilterType::Nearest)
        };
        
        // Save as JPEG - small file, fast!
        let filename = format!("{}.jpg", step_id);
        let filepath = output_dir.join(&filename);
        let rgb = DynamicImage::ImageRgba8(cropped).to_rgb8();
        rgb.save_with_format(&filepath, ImageFormat::Jpeg)
            .map_err(|e| anyhow::anyhow!("JPEG save failed: {}", e))?;
        
        Ok(filepath)
    }

    pub fn capture_and_save(output_dir: &PathBuf, step_id: &str) -> Result<PathBuf> {
        // Default: capture 400x300 region (or full screen if no coords)
        Self::capture_region(output_dir, step_id, None, None, 400)
    }

    fn fallback_screenshot(output_dir: &PathBuf, step_id: &str) -> Result<PathBuf> {
        // Fallback to screenshots crate if ImageMagick not available
        use screenshots::Screen;
        
        let screens = Screen::all()?;
        if screens.is_empty() {
            return Err(anyhow::anyhow!("No screens found"));
        }

        let screen = screens.first().unwrap();
        let image = screen.capture()?;
        
        // Get raw dimensions
        let width = image.width();
        let height = image.height();
        
        // Resize aggressively - 50% of original
        let new_width = width / 2;
        let new_height = height / 2;
        
        // Get PNG data with fast compression
        use screenshots::Compression as PngCompression;
        let png_data = image.to_png(Some(PngCompression::Fast))?;
        
        // Load and resize
        let img = image::load_from_memory(&png_data)?;
        let resized = imageops::resize(&img, new_width, new_height, imageops::FilterType::Nearest); // Fastest filter
        
        // Save as JPEG
        let filename = format!("{}.jpg", step_id);
        let filepath = output_dir.join(&filename);
        let rgb = DynamicImage::ImageRgba8(resized).to_rgb8();
        rgb.save_with_format(&filepath, ImageFormat::Jpeg)?;
        
        Ok(filepath)
    }
}

#[cfg(not(target_os = "linux"))]
pub struct ScreenshotCapturer;

#[cfg(not(target_os = "linux"))]
impl ScreenshotCapturer {
    pub fn capture_and_save(output_dir: &PathBuf, step_id: &str) -> Result<PathBuf> {
        // Fallback for non-Linux platforms
        use screenshots::Screen;
        
        std::fs::create_dir_all(output_dir)?;
        let screens = Screen::all()?;
        if screens.is_empty() {
            return Err(anyhow::anyhow!("No screens found"));
        }

        let screen = screens.first().unwrap();
        let image = screen.capture()?;
        
        let width = image.width();
        let height = image.height();
        let new_width = width / 2;
        let new_height = height / 2;
        
        use screenshots::Compression as PngCompression;
        let png_data = image.to_png(Some(PngCompression::Fast))?;
        let img = image::load_from_memory(&png_data)?;
        let resized = imageops::resize(&img, new_width, new_height, imageops::FilterType::Nearest);
        
        let filename = format!("{}.jpg", step_id);
        let filepath = output_dir.join(&filename);
        let rgb = DynamicImage::ImageRgba8(resized).to_rgb8();
        rgb.save_with_format(&filepath, ImageFormat::Jpeg)?;
        
        Ok(filepath)
    }
}
