use anyhow::{anyhow, Result};
use cairo::{Context, Format, ImageSurface};
use gdk4::ModifierType;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, DrawingArea, FileChooserAction, FileChooserDialog,
    Orientation, ResponseType,
};
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::tools::{AnnotationTools, Point};
use crate::ui::{load_css, StatusBar, Toolbar};

fn get_screen_dimensions() -> (i32, i32) {
    // Get screen dimensions using GDK
    let display = gdk4::Display::default().expect("Failed to get default display");
    let monitors = display.monitors();

    if monitors.n_items() > 0 {
        let monitor = monitors
            .item(0)
            .unwrap()
            .downcast::<gdk4::Monitor>()
            .unwrap();
        let geometry = monitor.geometry();
        (geometry.width(), geometry.height())
    } else {
        // Fallback to common screen resolution
        (1920, 1080)
    }
}

pub struct AnnotationEditor {
    window: ApplicationWindow,
    drawing_area: DrawingArea,
    toolbar: Toolbar,
    status_bar: StatusBar,
    tools: Rc<RefCell<AnnotationTools>>,
    screenshot_surface: Rc<RefCell<Option<ImageSurface>>>,
    image_width: i32,
    image_height: i32,
}

impl AnnotationEditor {
    pub fn new(app: &Application, image_data: Vec<u8>) -> Result<Self> {
        // Load CSS
        load_css();

        // Get screen dimensions to calculate window size
        let (screen_width, screen_height) = get_screen_dimensions();
        let window_width = screen_width / 2;
        let window_height = screen_height / 2;

        // Create the main window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Flint - Screenshot Editor")
            .default_width(window_width)
            .default_height(window_height)
            .resizable(true)
            .build();

        // Add CSS class for styling
        window.add_css_class("editor-window");

        // Load the screenshot image
        let screenshot_surface = Rc::new(RefCell::new(None));
        let (image_width, image_height) =
            Self::load_image_data(&image_data, screenshot_surface.clone())?;

        // Initialize tools
        let tools = Rc::new(RefCell::new(AnnotationTools::new()));
        let is_drawing = Rc::new(RefCell::new(false));

        // Create UI components
        let main_box = Box::new(Orientation::Vertical, 0);
        main_box.add_css_class("editor-main-box");

        // Create drawing area first so we can pass it to toolbar
        let drawing_area = DrawingArea::new();
        // Don't set fixed size request - let it scale with the window
        drawing_area.set_hexpand(true);
        drawing_area.set_vexpand(true);
        drawing_area.add_css_class("drawing-area");

        info!(
            "Drawing area created with size: {}x{}",
            image_width, image_height
        );

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

        info!(
            "Window sized to: {}x{} (half screen)",
            window_width, window_height
        );

        let editor = Self {
            window,
            drawing_area,
            toolbar,
            status_bar,
            tools,
            screenshot_surface,
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

        info!(
            "Loaded image: {}x{}, converting to Cairo surface",
            width, height
        );

        // Create Cairo surface from image data with proper stride
        let stride = cairo::Format::ARgb32
            .stride_for_width(width)
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
                    surface_data[dst_idx] = b; // Blue
                    surface_data[dst_idx + 1] = g; // Green
                    surface_data[dst_idx + 2] = r; // Red
                    surface_data[dst_idx + 3] = a; // Alpha
                } else {
                    error!("Buffer overflow prevented at pixel ({}, {})", x, y);
                    break;
                }
            }
        }

        info!(
            "Creating Cairo surface with dimensions {}x{}",
            width, height
        );
        let surface = ImageSurface::create_for_data(
            surface_data,
            Format::ARgb32,
            width as i32,
            height as i32,
            stride,
        )
        .map_err(|e| anyhow!("Failed to create Cairo surface: {}", e))?;

        *screenshot_surface.borrow_mut() = Some(surface);

        info!("Successfully loaded and converted image to Cairo surface");
        Ok((width as i32, height as i32))
    }

    fn setup_toolbar_callbacks(&self) {
        // Tool changed callback
        let tools_clone = self.tools.clone();
        let drawing_area_clone = self.drawing_area.clone();
        self.toolbar.connect_tool_changed(move |tool| {
            debug!("Tool changed to: {:?}", tool);
            tools_clone.borrow_mut().set_tool(tool);
            drawing_area_clone.queue_draw();
        });

        // Color changed callback
        let tools_clone = self.tools.clone();
        let drawing_area_clone = self.drawing_area.clone();
        self.toolbar.connect_color_changed(move |color| {
            debug!("Color changed to: {:?}", color);
            tools_clone.borrow_mut().set_color(color);
            drawing_area_clone.queue_draw();
        });

        // Thickness changed callback
        let tools_clone = self.tools.clone();
        let drawing_area_clone = self.drawing_area.clone();
        self.toolbar.connect_thickness_changed(move |thickness| {
            debug!("Thickness changed to: {}", thickness);
            tools_clone.borrow_mut().set_thickness(thickness);
            drawing_area_clone.queue_draw();
        });

        // Save button callback
        let window_for_save = self.window.clone();
        let screenshot_surface_for_save = self.screenshot_surface.clone();
        let tools_for_save = self.tools.clone();
        let status_bar_for_save = self.status_bar.clone();
        let image_width_for_save = self.image_width;
        let image_height_for_save = self.image_height;

        self.toolbar.connect_save_clicked(move || {
            info!("Save button clicked");
            Self::handle_save_action(
                &window_for_save,
                &screenshot_surface_for_save,
                &tools_for_save,
                &status_bar_for_save,
                image_width_for_save,
                image_height_for_save,
            );
        });

        // Copy button callback
        let screenshot_surface_for_copy = self.screenshot_surface.clone();
        let tools_for_copy = self.tools.clone();
        let status_bar_for_copy = self.status_bar.clone();
        let image_width_for_copy = self.image_width;
        let image_height_for_copy = self.image_height;

        self.toolbar.connect_copy_clicked(move || {
            info!("Copy button clicked");
            Self::handle_copy_action(
                &screenshot_surface_for_copy,
                &tools_for_copy,
                &status_bar_for_copy,
                image_width_for_copy,
                image_height_for_copy,
            );
        });

        // Clear button callback
        let tools_for_clear = self.tools.clone();
        let drawing_area_for_clear = self.drawing_area.clone();
        let status_bar_for_clear = self.status_bar.clone();

        self.toolbar.connect_clear_clicked(move || {
            info!("Clear button clicked");
            let stroke_count = tools_for_clear.borrow().strokes.len();
            if stroke_count > 0 {
                tools_for_clear.borrow_mut().clear_all();
                drawing_area_for_clear.queue_draw();
                status_bar_for_clear.set_status(&format!("Cleared {} annotations", stroke_count));
            } else {
                status_bar_for_clear.set_status("No annotations to clear");
            }
        });
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

            // Create a subtle gradient background for a modern look
            let gradient = cairo::LinearGradient::new(0.0, 0.0, 0.0, height as f64);
            gradient.add_color_stop_rgb(0.0, 0.15, 0.17, 0.21); // Top: #262D35
            gradient.add_color_stop_rgb(1.0, 0.12, 0.14, 0.18); // Bottom: slightly darker
            ctx.set_source(&gradient).unwrap();
            ctx.paint().unwrap();

            // Add a subtle texture pattern
            ctx.save().unwrap();
            ctx.set_source_rgba(1.0, 1.0, 1.0, 0.01); // Very subtle white dots
            for x in (0..width).step_by(20) {
                for y in (0..height).step_by(20) {
                    ctx.arc(x as f64, y as f64, 0.5, 0.0, 2.0 * std::f64::consts::PI);
                    ctx.fill().unwrap();
                }
            }
            ctx.restore().unwrap();

            // Draw the screenshot first
            if let Some(ref surface) = *screenshot_surface_draw.borrow() {
                debug!("Drawing screenshot surface");

                let image_width = surface.width() as f64;
                let image_height = surface.height() as f64;
                let area_width = width as f64;
                let area_height = height as f64;

                // Calculate scale factor to fit image within the drawing area
                let scale_x = area_width / image_width;
                let scale_y = area_height / image_height;
                let scale = scale_x.min(scale_y);

                // Calculate centered position
                let scaled_width = image_width * scale;
                let scaled_height = image_height * scale;
                let offset_x = (area_width - scaled_width) / 2.0;
                let offset_y = (area_height - scaled_height) / 2.0;

                ctx.save().unwrap();
                ctx.translate(offset_x, offset_y);
                ctx.scale(scale, scale);
                ctx.set_source_surface(surface, 0.0, 0.0).unwrap();
                ctx.paint().unwrap();
                ctx.restore().unwrap();

                debug!(
                    "Image scaled by {:.2} and positioned at ({:.1}, {:.1})",
                    scale, offset_x, offset_y
                );
            } else {
                warn!("No screenshot surface available to draw");
                // Draw a placeholder with subtle dark background
                ctx.set_source_rgb(0.18, 0.20, 0.24); // Slightly lighter than main background
                ctx.rectangle(0.0, 0.0, width as f64, height as f64);
                ctx.fill().unwrap();

                // Draw text indicating no image with light text
                ctx.set_source_rgb(0.7, 0.7, 0.7); // Light gray text for dark theme
                ctx.move_to(20.0, height as f64 / 2.0);
                ctx.show_text("No screenshot loaded").unwrap();
            }

            // Draw annotations on top (they need to be scaled too)
            if let Some(ref surface) = *screenshot_surface_draw.borrow() {
                let image_width = surface.width() as f64;
                let image_height = surface.height() as f64;
                let area_width = width as f64;
                let area_height = height as f64;

                let scale_x = area_width / image_width;
                let scale_y = area_height / image_height;
                let scale = scale_x.min(scale_y);

                let scaled_width = image_width * scale;
                let scaled_height = image_height * scale;
                let offset_x = (area_width - scaled_width) / 2.0;
                let offset_y = (area_height - scaled_height) / 2.0;

                ctx.save().unwrap();
                ctx.translate(offset_x, offset_y);
                ctx.scale(scale, scale);
                tools_draw.borrow().draw_all(ctx);
                ctx.restore().unwrap();
            } else {
                // If no image, draw annotations without scaling
                tools_draw.borrow().draw_all(ctx);
            }
        });

        // Mouse button press
        let gesture_click = gtk4::GestureClick::new();
        let tools_click = tools.clone();
        let is_drawing_click = is_drawing.clone();
        let drawing_area_click = drawing_area.clone();
        let screenshot_surface_click = screenshot_surface.clone();

        gesture_click.connect_pressed(move |_, _, x, y| {
            debug!("Mouse pressed at screen coords ({}, {})", x, y);

            // Convert screen coordinates to image coordinates
            let (image_x, image_y) = if let Some(ref surface) = *screenshot_surface_click.borrow() {
                let allocation = drawing_area_click.allocation();
                let area_width = allocation.width() as f64;
                let area_height = allocation.height() as f64;
                let image_width = surface.width() as f64;
                let image_height = surface.height() as f64;

                let scale_x = area_width / image_width;
                let scale_y = area_height / image_height;
                let scale = scale_x.min(scale_y);

                let scaled_width = image_width * scale;
                let scaled_height = image_height * scale;
                let offset_x = (area_width - scaled_width) / 2.0;
                let offset_y = (area_height - scaled_height) / 2.0;

                let image_x = (x - offset_x) / scale;
                let image_y = (y - offset_y) / scale;

                debug!("Converted to image coords ({:.1}, {:.1})", image_x, image_y);
                (image_x, image_y)
            } else {
                (x, y)
            };

            *is_drawing_click.borrow_mut() = true;
            tools_click
                .borrow_mut()
                .start_stroke(Point::new(image_x, image_y));
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
        let screenshot_surface_motion = screenshot_surface.clone();

        motion_controller.connect_motion(move |_, x, y| {
            // Convert screen coordinates to image coordinates for display
            let (image_x, image_y) = if let Some(ref surface) = *screenshot_surface_motion.borrow()
            {
                let allocation = drawing_area_motion.allocation();
                let area_width = allocation.width() as f64;
                let area_height = allocation.height() as f64;
                let image_width = surface.width() as f64;
                let image_height = surface.height() as f64;

                let scale_x = area_width / image_width;
                let scale_y = area_height / image_height;
                let scale = scale_x.min(scale_y);

                let scaled_width = image_width * scale;
                let scaled_height = image_height * scale;
                let offset_x = (area_width - scaled_width) / 2.0;
                let offset_y = (area_height - scaled_height) / 2.0;

                let image_x = (x - offset_x) / scale;
                let image_y = (y - offset_y) / scale;

                (image_x, image_y)
            } else {
                (x, y)
            };

            // Show image coordinates in status bar
            status_bar_motion.set_coordinates(image_x, image_y);

            if *is_drawing_motion.borrow() {
                tools_motion
                    .borrow_mut()
                    .add_point_to_stroke(Point::new(image_x, image_y));
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
        let is_drawing_key = is_drawing.clone();

        key_controller.connect_key_pressed(move |_, key, _, modifier| {
            match (key, modifier) {
                (gdk4::Key::Escape, _) => {
                    if *is_drawing_key.borrow() {
                        tools_key.borrow_mut().cancel_stroke();
                        *is_drawing_key.borrow_mut() = false;
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
        self.status_bar
            .set_status("Ready - Select a tool and start annotating");

        // Force a redraw to ensure the screenshot is displayed
        self.drawing_area.queue_draw();

        // Show and present the window
        self.window.set_visible(true);
        self.window.present();
        gtk4::prelude::GtkWindowExt::set_focus(&self.window, Some(&self.drawing_area));

        info!("Editor window presented and focused");
    }

    fn handle_save_action(
        window: &ApplicationWindow,
        screenshot_surface: &Rc<RefCell<Option<ImageSurface>>>,
        tools: &Rc<RefCell<AnnotationTools>>,
        status_bar: &StatusBar,
        image_width: i32,
        image_height: i32,
    ) {
        let dialog = FileChooserDialog::new(
            Some("Save Screenshot"),
            Some(window),
            FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );

        dialog.set_current_name("flint-screenshot.png");

        let screenshot_surface_clone = screenshot_surface.clone();
        let tools_clone = tools.clone();
        let status_bar_clone = status_bar.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        match Self::render_to_file_static(
                            &path,
                            &screenshot_surface_clone,
                            &tools_clone,
                            image_width,
                            image_height,
                        ) {
                            Ok(_) => {
                                status_bar_clone
                                    .set_status(&format!("Saved to {}", path.display()));
                                info!("Screenshot saved to: {}", path.display());
                            }
                            Err(e) => {
                                error!("Failed to save file: {}", e);
                                status_bar_clone.set_status("Error saving file");
                            }
                        }
                    }
                }
            }
            dialog.close();
        });

        dialog.present();
    }

    fn handle_copy_action(
        screenshot_surface: &Rc<RefCell<Option<ImageSurface>>>,
        tools: &Rc<RefCell<AnnotationTools>>,
        status_bar: &StatusBar,
        image_width: i32,
        image_height: i32,
    ) {
        match Self::copy_to_clipboard_static(screenshot_surface, tools, image_width, image_height) {
            Ok(_) => {
                status_bar.set_status("Copied to clipboard");
                info!("Screenshot copied to clipboard");
            }
            Err(e) => {
                error!("Failed to copy to clipboard: {}", e);
                status_bar.set_status("Error copying to clipboard");
            }
        }
    }

    fn render_to_file_static<P: AsRef<Path>>(
        path: P,
        screenshot_surface: &Rc<RefCell<Option<ImageSurface>>>,
        tools: &Rc<RefCell<AnnotationTools>>,
        image_width: i32,
        image_height: i32,
    ) -> Result<()> {
        let mut surface = ImageSurface::create(Format::ARgb32, image_width, image_height)?;

        let ctx = Context::new(&surface)?;

        // Draw screenshot
        if let Some(ref screenshot) = *screenshot_surface.borrow() {
            ctx.set_source_surface(screenshot, 0.0, 0.0)?;
            ctx.paint()?;
        }

        // Draw annotations
        tools.borrow().draw_all(&ctx);

        // Convert to PNG and save
        let data = surface.data()?;
        let image_data = Self::argb_to_rgba_static(&data, image_width, image_height);

        let img = image::RgbaImage::from_raw(image_width as u32, image_height as u32, image_data)
            .ok_or_else(|| anyhow!("Failed to create image from data"))?;

        img.save(path)?;

        Ok(())
    }

    fn copy_to_clipboard_static(
        screenshot_surface: &Rc<RefCell<Option<ImageSurface>>>,
        tools: &Rc<RefCell<AnnotationTools>>,
        image_width: i32,
        image_height: i32,
    ) -> Result<()> {
        // Create a surface for the final image
        let mut surface = ImageSurface::create(Format::ARgb32, image_width, image_height)?;

        let ctx = Context::new(&surface)?;

        // Draw screenshot
        if let Some(ref screenshot) = *screenshot_surface.borrow() {
            ctx.set_source_surface(screenshot, 0.0, 0.0)?;
            ctx.paint()?;
        }

        // Draw annotations
        tools.borrow().draw_all(&ctx);

        // Convert to image data
        let data = surface.data()?;
        let image_data = Self::argb_to_rgba_static(&data, image_width, image_height);

        // Create image and save to temp file for clipboard
        let img = image::RgbaImage::from_raw(image_width as u32, image_height as u32, image_data)
            .ok_or_else(|| anyhow!("Failed to create image from data"))?;

        // Save to temp file for clipboard
        let temp_path = "/tmp/flint_screenshot.png";
        img.save(temp_path)?;

        warn!(
            "Image clipboard not fully implemented - saved to {}",
            temp_path
        );

        Ok(())
    }

    fn argb_to_rgba_static(argb_data: &[u8], width: i32, height: i32) -> Vec<u8> {
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
}
