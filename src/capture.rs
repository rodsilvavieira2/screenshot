use anyhow::{anyhow, Result};
use image::GenericImageView;
use log::{debug, info, warn};
use std::path::PathBuf;

pub struct ScreenshotCapture {
    pub use_portal: bool,
}

impl ScreenshotCapture {
    pub fn new() -> Self {
        // Check if we're running on Wayland and if portal is available
        let use_portal = Self::detect_portal_availability();

        Self { use_portal }
    }

    fn detect_portal_availability() -> bool {
        // Check for Wayland session
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            debug!("Wayland session detected");
            return true;
        }

        // Check for XDG portal even on X11
        if std::env::var("XDG_CURRENT_DESKTOP").is_ok() {
            debug!("XDG desktop session detected, will try portal");
            return true;
        }

        debug!("No portal environment detected, falling back to X11");
        false
    }

    pub fn take_screenshot_blocking(&self) -> Result<Vec<u8>> {
        info!("Starting screenshot capture process");

        if self.use_portal {
            info!("Attempting screenshot via portal");
            match self.take_screenshot_portal_blocking() {
                Ok(data) => {
                    info!("Portal screenshot successful");
                    return Ok(data);
                }
                Err(e) => {
                    warn!("Portal screenshot failed: {}, falling back to X11", e);
                    // Continue to X11 fallback
                }
            }
        }

        info!("Taking screenshot via X11 fallback");
        self.take_screenshot_x11_blocking()
            .map_err(|e| anyhow!("Screenshot capture failed: {}. Please ensure you're running in a graphical environment with proper permissions.", e))
    }

    pub fn take_screenshot_region_blocking(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<Vec<u8>> {
        info!(
            "Starting region screenshot capture process: {}x{} at ({}, {})",
            width, height, x, y
        );

        // For now, we'll capture full screen and crop the region
        // In a full implementation, we could use X11 region capture or portal region selection
        let full_screenshot = self.take_screenshot_blocking()?;
        self.crop_image_region(&full_screenshot, x, y, width, height)
    }

    fn take_screenshot_portal_blocking(&self) -> Result<Vec<u8>> {
        info!("Attempting to use portal for screenshot capture");

        // Add delay to ensure UI is hidden
        std::thread::sleep(std::time::Duration::from_millis(200));

        // For V1.0, we'll use a simplified approach
        // In a full implementation, we'd use the portal properly
        warn!("Portal screenshot not fully implemented in V1.0 - falling back to X11");
        self.take_screenshot_x11_blocking()
    }

    fn take_screenshot_x11_blocking(&self) -> Result<Vec<u8>> {
        info!("Using X11 fallback for screenshot capture");

        // Add delay to ensure capture window is hidden
        std::thread::sleep(std::time::Duration::from_millis(300));

        // Use screenshots crate for X11 fallback
        let screens = screenshots::Screen::all()
            .map_err(|e| anyhow!("Failed to enumerate screens: {}. Make sure you're running in a graphical environment.", e))?;

        if screens.is_empty() {
            return Err(anyhow!("No screens found. Make sure you're running in a graphical environment with a display."));
        }

        // For V1.0, we only capture the primary screen (full screen)
        let screen = &screens[0];
        info!(
            "Capturing screen: {}x{}",
            screen.display_info.width, screen.display_info.height
        );

        let image = screen.capture()
            .map_err(|e| anyhow!("Failed to capture screen: {}. This might be due to permissions or running in a headless environment.", e))?;

        // Convert screenshots::Image to PNG bytes
        let width = image.width() as u32;
        let height = image.height() as u32;

        if width == 0 || height == 0 {
            return Err(anyhow!("Invalid screen dimensions: {}x{}", width, height));
        }

        let rgba_data = image.rgba();

        if rgba_data.is_empty() {
            return Err(anyhow!("Screenshot capture returned empty image data"));
        }

        info!("Converting {}x{} image to PNG", width, height);

        // Create image::RgbaImage and save as PNG
        let img =
            image::RgbaImage::from_raw(width, height, rgba_data.clone()).ok_or_else(|| {
                anyhow!(
                    "Failed to create image from raw data. Image size: {}x{}, data length: {}",
                    width,
                    height,
                    rgba_data.len()
                )
            })?;

        let mut buffer = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut buffer),
            image::ImageOutputFormat::Png,
        )
        .map_err(|e| anyhow!("Failed to convert image to PNG: {}", e))?;

        if buffer.is_empty() {
            return Err(anyhow!("PNG conversion resulted in empty buffer"));
        }

        info!("Screenshot converted to PNG, {} bytes", buffer.len());
        Ok(buffer)
    }

    fn crop_image_region(
        &self,
        image_data: &[u8],
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<Vec<u8>> {
        info!(
            "Cropping image region: {}x{} at ({}, {})",
            width, height, x, y
        );

        // Load the image from bytes
        let image = image::load_from_memory(image_data)
            .map_err(|e| anyhow!("Failed to load image for cropping: {}", e))?;

        let (img_width, img_height) = image.dimensions();
        info!("Original image dimensions: {}x{}", img_width, img_height);

        // Validate crop bounds
        let crop_x = x.max(0) as u32;
        let crop_y = y.max(0) as u32;
        let crop_width = width.min(img_width as i32 - x).max(1) as u32;
        let crop_height = height.min(img_height as i32 - y).max(1) as u32;

        if crop_x >= img_width || crop_y >= img_height {
            return Err(anyhow!("Crop region is outside image bounds"));
        }

        info!(
            "Adjusted crop region: {}x{} at ({}, {})",
            crop_width, crop_height, crop_x, crop_y
        );

        // Crop the image
        let cropped = image.crop_imm(crop_x, crop_y, crop_width, crop_height);

        // Convert back to PNG bytes
        let mut buffer = Vec::new();
        cropped
            .write_to(
                &mut std::io::Cursor::new(&mut buffer),
                image::ImageOutputFormat::Png,
            )
            .map_err(|e| anyhow!("Failed to convert cropped image to PNG: {}", e))?;

        info!("Cropped image converted to PNG, {} bytes", buffer.len());
        Ok(buffer)
    }
}

impl Default for ScreenshotCapture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portal_detection() {
        // This test just ensures the detection doesn't panic
        let _use_portal = ScreenshotCapture::detect_portal_availability();
    }
}
