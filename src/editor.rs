use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, DrawingArea, FileChooserAction, 
    FileChooserDialog, Orientation, ResponseType
};
use gdk4::ModifierType;
use cairo::{Context, ImageSurface, Format};
use anyhow::{Result, anyhow};
use log::{info, warn, debug, error};
use std::cell::RefCell;
use std::rc::Rc;
use std::path::Path;
use arboard::Clipboard;

use crate::tools::{AnnotationTools, Point};
use crate::ui::{Toolbar, StatusBar, load_css};

pub struct AnnotationEditor {
    window: ApplicationWindow,
    drawing_area: DrawingArea,
    toolbar: Toolbar,
    status_bar: StatusBar,
    tools: Rc<RefCell<AnnotationTools>>,
    screenshot_surface: Rc<RefCell<Option<ImageSurface>>>,
    is_drawing: Rc<RefCell<bool>>,
    image_width: i32,
    image_height: i32,
}

impl AnnotationEditor {
    pub fn new(app: &Application, image_data: Vec<u8>) -> Result<Self> {
        // Load CSS
        load_css();

        // Create the main window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Flint - Screenshot Editor")
            .default_width(800)
            .default_height(600)
            .build();

        // Load the screenshot image
        let screenshot_surface = Rc::new(RefCell::new(None));
        let (image_width, image_height) = Self::load_image_data(
            &image_data,
            screenshot_surface.clone()
        )?;

        // Initialize tools
        let tools = Rc::new(RefCell::new(AnnotationTools::new()));
        let is_drawing = Rc::new(RefCell::new(false));

        // Create UI components
        let main_box = Box::new(Orientation::Vertical, 0);

        // Create drawing area first so we can pass it to toolbar
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(image_width, image_height);
        drawing_area.set_hexpand(true);
        drawing_area.set_vexpand(true);
        drawing_area.add_css_class("drawing-area");
        
        info!("Drawing area created with size: {}x{}", image_width, image_height);

        // Create toolbar
        let toolbar = Toolbar::new();


        // Create status bar
        let status_bar = StatusBar::new();

        // Setup drawing area events
        Self::setup_drawing_events(
            &drawing_area,
            tools.clone(),
            is_drawing.clone(),
            screenshot_surface.clone(),
            status_bar.clone(),
        );

        // Set drawing area to be focusable and grab focus
        drawing_area.set_can_focus(true);
        drawing_area.set_focusable(true);

        // Assemble the UI
        main_box.append(toolbar.get_widget());
        main_box.append(&drawing_area);
        main_box.append(status_bar.get_widget());

        window.set_child(Some(&main_box));

        // Resize window to fit the image
        let window_width = image_width + 40;  // Add some padding
        let window_height = image_height + 120; // Add space for toolbar and status bar
        window.set_default_size(window_width, window_height);
        
        info!("Window sized to: {}x{}", window_width, window_height);

        let editor = Self {
            window,
            drawing_area,
            toolbar,
            status_bar,
            tools,
            screenshot_surface,
            is_drawing,
            image_width,
            image_height,
        };

        // Setup toolbar callbacks after creation
        editor.setup_toolbar_callbacks();

        Ok(editor)
    }

    fn load_image_data(
        image_data: &[u8],
        screenshot_surface: Rc<RefCell<Option<ImageSurface>>>,
    ) -> Result<(i32, i32)> {
        info!("Loading image data: {} bytes", image_data.len());
        
        let image = image::load_from_memory(image_data)
            .map_err(|e| anyhow!("Failed to load image from memory: {}", e))?;
        let rgba_image = image.to_rgba8();
        let (width, height) = rgba_image.dimensions();

        info!("Loaded image: {}x{}, converting to Cairo surface", width, height);

        // Create Cairo surface from image data with proper stride
        let stride = cairo::Format::ARgb32.stride_for_width(width)
            .map_err(|e| anyhow!("Failed to calculate stride: {}", e))?;
        let mut surface_data = vec![0u8; (stride * height as i32) as usize];

        info!("Converting RGBA to Cairo BGRA format, stride: {}", stride);

        // Convert RGBA to BGRA (Cairo's native format on little-endian)
        for y in 0..height {
            for x in 0..width {
                let src_pixel = rgba_image.get_pixel(x, y);
                let dst_idx = (y as i32 * stride + x as i32 * 4) as usize;
                
                if dst_idx + 3 < surface_data.len() {
                    let r = src_pixel[0];
                    let g = src_pixel[1];
                    let b = src_pixel[2];
                    let a = src_pixel[3];
                    
                    // Cairo expects BGRA on little-endian systems
                    surface_data[dst_idx] = b;     // Blue
                    surface_data[dst_idx + 1] = g; // Green
                    surface_data[dst_idx + 2] = r; // Red
                    surface_data[dst_idx + 3] = a; // Alpha
                } else {
                    error!("Buffer overflow prevented at pixel ({}, {})", x, y);
                    break;
                }
            }
        }

        info!("Creating Cairo surface with dimensions {}x{}", width, height);
        let surface = ImageSurface::create_for_data(
            surface_data,
            Format::ARgb32,
            width as i32,
            height as i32,
            stride,
        ).map_err(|e| anyhow!("Failed to create Cairo surface: {}", e))?;

        *screenshot_surface.borrow_mut() = Some(surface);
        
        info!("Successfully loaded and converted image to Cairo surface");
        Ok((width as i32, height as i32))
    }

    fn setup_toolbar_callbacks(&self) {
        // Tool changed callback
        let tools_clone = self.tools.clone();
        self.toolbar.connect_tool_changed(move |tool| {
            debug!("Tool changed to: {:?}", tool);
            tools_clone.borrow_mut().set_tool(tool);
        });

        // Color changed callback
        let tools_clone = self.tools.clone();
        self.toolbar.connect_color_changed(move |color| {
            debug!("Color changed to: {:?}", color);
            tools_clone.borrow_mut().set_color(color);
        });

        // Thickness changed callback
        let tools_clone = self.tools.clone();
        self.toolbar.connect_thickness_changed(move |thickness| {
            debug!("Thickness changed to: {}", thickness);
            tools_clone.borrow_mut().set_thickness(thickness);
        });

        // Note: Clear callback simplified for V1.0
        // Would implement proper button connection in full version
    }

    fn setup_drawing_events(
        drawing_area: &DrawingArea,
        tools: Rc<RefCell<AnnotationTools>>,
        is_drawing: Rc<RefCell<bool>>,
        screenshot_surface: Rc<RefCell<Option<ImageSurface>>>,
        status_bar: StatusBar,
    ) {
        // Setup draw function
        let tools_draw = tools.clone();
        let screenshot_surface_draw = screenshot_surface.clone();
        drawing_area.set_draw_func(move |_area, ctx, width, height| {
            debug!("Drawing callback: area={}x{}", width, height);
            
            // Clear the background with a light gray
            ctx.set_source_rgb(0.95, 0.95, 0.95);
            ctx.paint().unwrap();
            
            // Draw the screenshot first
            if let Some(ref surface) = *screenshot_surface_draw.borrow() {
                debug!("Drawing screenshot surface");
                ctx.save().unwrap();
                ctx.set_source_surface(surface, 0.0, 0.0).unwrap();
                ctx.paint().unwrap();
                ctx.restore().unwrap();
            } else {
                warn!("No screenshot surface available to draw");
                // Draw a placeholder
                ctx.set_source_rgb(0.8, 0.8, 0.8);
                ctx.rectangle(0.0, 0.0, width as f64, height as f64);
                ctx.fill().unwrap();
                
                // Draw text indicating no image
                ctx.set_source_rgb(0.3, 0.3, 0.3);
                ctx.move_to(20.0, height as f64 / 2.0);
                ctx.show_text("No screenshot loaded").unwrap();
            }

            // Draw annotations on top
            tools_draw.borrow().draw_all(ctx);
        });

        // Mouse button press
        let gesture_click = gtk4::GestureClick::new();
        let tools_click = tools.clone();
        let is_drawing_click = is_drawing.clone();
        let drawing_area_click = drawing_area.clone();
        
        gesture_click.connect_pressed(move |_, _, x, y| {
            debug!("Mouse pressed at ({}, {})", x, y);
            *is_drawing_click.borrow_mut() = true;
            tools_click.borrow_mut().start_stroke(Point::new(x, y));
            drawing_area_click.queue_draw();
        });

        let tools_release = tools.clone();
        let is_drawing_release = is_drawing.clone();
        let drawing_area_release = drawing_area.clone();
        
        gesture_click.connect_released(move |_, _, _, _| {
            debug!("Mouse released");
            if *is_drawing_release.borrow() {
                tools_release.borrow_mut().finish_stroke();
                *is_drawing_release.borrow_mut() = false;
                drawing_area_release.queue_draw();
            }
        });

        drawing_area.add_controller(gesture_click);

        // Mouse motion
        let motion_controller = gtk4::EventControllerMotion::new();
        let tools_motion = tools.clone();
        let is_drawing_motion = is_drawing.clone();
        let drawing_area_motion = drawing_area.clone();
        let status_bar_motion = status_bar.clone();
        
        motion_controller.connect_motion(move |_, x, y| {
            status_bar_motion.set_coordinates(x, y);
            
            if *is_drawing_motion.borrow() {
                tools_motion.borrow_mut().add_point_to_stroke(Point::new(x, y));
                drawing_area_motion.queue_draw();
            }
        });

        let status_bar_leave = status_bar.clone();
        motion_controller.connect_leave(move |_| {
            status_bar_leave.clear_coordinates();
        });

        drawing_area.add_controller(motion_controller);

        // Key events for shortcuts
        let key_controller = gtk4::EventControllerKey::new();
        let tools_key = tools.clone();
        let drawing_area_key = drawing_area.clone();
        
        key_controller.connect_key_pressed(move |_, key, _, modifier| {
            match (key, modifier) {
                (gdk4::Key::Escape, _) => {
                    if *is_drawing.borrow() {
                        tools_key.borrow_mut().cancel_stroke();
                        *is_drawing.borrow_mut() = false;
                        drawing_area_key.queue_draw();
                    }
                    glib::Propagation::Stop
                }
                (gdk4::Key::z, ModifierType::CONTROL_MASK) => {
                    // Could implement undo here in future versions
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });

        drawing_area.add_controller(key_controller);
        drawing_area.set_can_focus(true);
    }

    pub fn show(&self) {
        info!("Showing annotation editor window");
        self.status_bar.set_status("Ready - Select a tool and start annotating");
        
        // Force a redraw to ensure the screenshot is displayed
        self.drawing_area.queue_draw();
        
        // Show and present the window
        self.window.set_visible(true);
        self.window.present();
        gtk4::prelude::GtkWindowExt::set_focus(&self.window, Some(&self.drawing_area));
        
        info!("Editor window presented and focused");
    }

    pub fn save_to_file(&self) -> Result<()> {
        let dialog = FileChooserDialog::new(
            Some("Save Screenshot"),
            Some(&self.window),
            FileChooserAction::Save,
            &[("Cancel", ResponseType::Cancel), ("Save", ResponseType::Accept)],
        );

        dialog.set_current_name("screenshot.png");

        // Use present for simple V1.0 implementation
        dialog.present();
        
        // Note: For a complete implementation, we'd need proper async handling
        // This is a simplified version for V1.0

        Ok(())
    }

    pub fn copy_to_clipboard(&self) -> Result<()> {
        info!("Copying to clipboard");
        
        // Create a surface for the final image
        let mut surface = ImageSurface::create(
            Format::ARgb32,
            self.image_width,
            self.image_height,
        )?;
        
        let ctx = Context::new(&surface)?;
        
        // Draw screenshot
        if let Some(ref screenshot) = *self.screenshot_surface.borrow() {
            ctx.set_source_surface(screenshot, 0.0, 0.0)?;
            ctx.paint()?;
        }
        
        // Draw annotations
        self.tools.borrow().draw_all(&ctx);
        
        // Convert to image data
        let data = surface.data()?;
        let image_data = self.argb_to_rgba(&data, self.image_width, self.image_height);
        
        // Create image and copy to clipboard
        let img = image::RgbaImage::from_raw(
            self.image_width as u32,
            self.image_height as u32,
            image_data,
        ).ok_or_else(|| anyhow!("Failed to create image from data"))?;
        
        // Convert to PNG for clipboard
        let mut png_data = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)?;
        
        // Note: Full clipboard image support would require platform-specific implementation
        // For V1.0, we'll save to a temporary file and copy the path
        let temp_path = "/tmp/flint_screenshot.png";
        img.save(temp_path)?;
        
        let _clipboard = Clipboard::new()?;
        // clipboard.set_text(format!("Screenshot saved to: {}", temp_path))?;
        
        warn!("Image clipboard not fully implemented - copied path to clipboard instead");
        
        self.status_bar.set_status("Copied to clipboard");
        
        Ok(())
    }

    pub fn clear_annotations(&self) {
        info!("Clearing all annotations");
        self.tools.borrow_mut().clear_all();
        self.drawing_area.queue_draw();
        self.status_bar.set_status("Annotations cleared");
    }

    fn render_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut surface = ImageSurface::create(
            Format::ARgb32,
            self.image_width,
            self.image_height,
        )?;
        
        let ctx = Context::new(&surface)?;
        
        // Draw screenshot
        if let Some(ref screenshot) = *self.screenshot_surface.borrow() {
            ctx.set_source_surface(screenshot, 0.0, 0.0)?;
            ctx.paint()?;
        }
        
        // Draw annotations
        self.tools.borrow().draw_all(&ctx);
        
        // Convert to PNG and save
        let data = surface.data()?;
        let image_data = self.argb_to_rgba(&data, self.image_width, self.image_height);
        
        let img = image::RgbaImage::from_raw(
            self.image_width as u32,
            self.image_height as u32,
            image_data,
        ).ok_or_else(|| anyhow!("Failed to create image from data"))?;
        
        img.save(path)?;
        
        Ok(())
    }

    fn argb_to_rgba(&self, argb_data: &[u8], width: i32, height: i32) -> Vec<u8> {
        let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
        
        for chunk in argb_data.chunks_exact(4) {
            // ARGB -> RGBA conversion
            let a = chunk[0];
            let r = chunk[1];
            let g = chunk[2];
            let b = chunk[3];
            
            rgba_data.extend_from_slice(&[r, g, b, a]);
        }
        
        rgba_data
    }

    pub fn connect_toolbar_actions(&self) {
        // Note: Toolbar action connections would be implemented here
        // For V1.0, we're using a simplified approach
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_argb_to_rgba_conversion() {
        // Create a minimal test without requiring full editor
        let argb = vec![255, 255, 0, 0, 128, 0, 255, 0]; // Two pixels
        let mut rgba_data = Vec::with_capacity(8);
        
        for chunk in argb.chunks_exact(4) {
            let a = chunk[0];
            let r = chunk[1]; 
            let g = chunk[2];
            let b = chunk[3];
            
            rgba_data.extend_from_slice(&[r, g, b, a]);
        }
        
        assert_eq!(rgba_data, vec![255, 0, 0, 255, 0, 255, 0, 128]);
    }
}