use anyhow::Result;
use cairo;
use gdk4;
use gtk4::pango;
use gtk4::prelude::*;
use gtk4::{
    glib, Application, ApplicationWindow, Box, Button, DrawingArea, Label, ListBox, ListBoxRow,
    Orientation, PolicyType, ScrolledWindow, SelectionMode,
};
use image::GenericImageView;
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

mod capture;
mod editor;
mod tools;
mod ui;
mod window_manager;

use capture::ScreenshotCapture;
use editor::AnnotationEditor;

const APP_ID: &str = "com.flint.Screenshot";

fn main() -> Result<()> {
    env_logger::init();

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_capture_ui);

    let exit_code = app.run();

    std::process::exit(exit_code.into());
}

fn build_capture_ui(app: &Application) {
    // Create the main capture window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Flint - Screenshot Tool")
        .default_width(400)
        .default_height(200)
        .resizable(false)
        .build();

    // Create the main container
    let main_box = Box::new(Orientation::Vertical, 20);
    main_box.set_margin_start(30);
    main_box.set_margin_end(30);
    main_box.set_margin_top(30);
    main_box.set_margin_bottom(30);
    main_box.set_halign(gtk4::Align::Center);
    main_box.set_valign(gtk4::Align::Center);

    // Title label
    let title_label = Label::new(Some("Flint Screenshot Tool"));
    title_label.set_margin_bottom(10);

    // Description label
    let desc_label = Label::new(Some("Capture and annotate screenshots"));
    desc_label.set_margin_bottom(10);

    // Capture buttons container
    let button_box = Box::new(Orientation::Vertical, 10);

    // Full screenshot button
    let capture_button = Button::with_label("Screen");
    capture_button.set_size_request(200, 50);

    // Rectangle selection button
    let rect_button = Button::with_label("Selection");
    rect_button.set_size_request(200, 50);

    // Window selection button
    let window_button = Button::with_label("Window");
    window_button.set_size_request(200, 50);

    // Clone app for the callbacks
    let app_clone = app.clone();
    let window_clone = window.clone();
    let app_clone2 = app.clone();
    let window_clone2 = window.clone();
    let app_clone3 = app.clone();
    let window_clone3 = window.clone();

    // Full screenshot button callback
    capture_button.connect_clicked(move |_| {
        info!("Full screenshot button clicked");
        start_screenshot_capture(app_clone.clone(), window_clone.clone(), false);
    });

    // Rectangle selection button callback
    rect_button.connect_clicked(move |_| {
        info!("Rectangle selection button clicked");
        start_screenshot_capture(app_clone2.clone(), window_clone2.clone(), true);
    });

    // Window selection button callback
    window_button.connect_clicked(move |_| {
        info!("Window selection button clicked");
        start_window_selection_capture(app_clone3.clone(), window_clone3.clone());
    });

    // Keyboard shortcuts
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.connect_key_pressed(glib::clone!(@weak window => @default-return glib::Propagation::Proceed, move |_, key, _, _| {
        match key {
            gdk4::Key::Escape => {
                window.close();
                glib::Propagation::Stop
            }
            gdk4::Key::Return | gdk4::Key::space => {
                // Trigger capture on Enter or Space
                if let Some(capture_btn) = window.child()
                    .and_then(|w| w.downcast::<Box>().ok())
                    .and_then(|b| {
                        // Find the capture button in the box
                        let mut child = b.first_child();
                        while let Some(widget) = child {
                            if let Ok(button) = widget.clone().downcast::<Button>() {
                                if button.label().map_or(false, |l| l.contains("Take Screenshot")) {
                                    return Some(button);
                                }
                            }
                            child = widget.next_sibling();
                        }
                        None
                    })
                {
                    capture_btn.emit_clicked();
                }
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    }));

    window.add_controller(key_controller);

    // Add buttons to button container
    button_box.append(&capture_button);
    button_box.append(&rect_button);
    button_box.append(&window_button);

    // Add widgets to container
    main_box.append(&title_label);
    main_box.append(&desc_label);
    main_box.append(&button_box);

    window.set_child(Some(&main_box));

    // Show the window
    window.present();

    info!("Capture interface ready");
}

fn start_screenshot_capture(app: Application, window: ApplicationWindow, is_rectangle: bool) {
    // Hide the capture window
    window.set_visible(false);

    if is_rectangle {
        // Show rectangle selection overlay
        show_rectangle_selection(app, window);
    } else {
        // Proceed with full screenshot
        proceed_with_screenshot(app, window, None);
    }
}

fn show_rectangle_selection(app: Application, parent_window: ApplicationWindow) {
    // Hide parent window first and ensure it's completely hidden
    parent_window.set_visible(false);

    // Additional delay to ensure the capture window is fully hidden before preview capture
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        // Now capture the actual current screen state for preview (without the capture UI)
        let screen_info = get_screen_info_without_capture();
        let (preview_surface, original_png_data) =
            capture_current_screen_for_preview_with_data(screen_info.0, screen_info.1);

        // Create fullscreen overlay window for rectangle selection
        let overlay_window = ApplicationWindow::builder()
            .application(&app)
            .title("Select Rectangle Area")
            .default_width(screen_info.0)
            .default_height(screen_info.1)
            .decorated(false)
            .build();

        // Configure for Wayland compatibility
        overlay_window.set_modal(true);
        overlay_window.set_resizable(false);
        overlay_window.set_deletable(false);

        overlay_window.fullscreen();

        let drawing_area = DrawingArea::new();
        drawing_area.set_hexpand(true);
        drawing_area.set_vexpand(true);

        let selection_start = Rc::new(RefCell::new(None::<(f64, f64)>));
        let selection_end = Rc::new(RefCell::new(None::<(f64, f64)>));
        let is_selecting = Rc::new(RefCell::new(false));

        let selection_start_draw = selection_start.clone();
        let selection_end_draw = selection_end.clone();

        drawing_area.set_draw_func(move |_, ctx, width, height| {
            // Draw the preview pattern as background
            ctx.save().unwrap();
            ctx.scale(
                width as f64 / screen_info.0 as f64,
                height as f64 / screen_info.1 as f64,
            );
            ctx.set_source_surface(&preview_surface, 0.0, 0.0).unwrap();
            ctx.paint().unwrap();
            ctx.restore().unwrap();

            // Add a subtle dark overlay to indicate selection mode
            ctx.set_source_rgba(0.0, 0.0, 0.0, 0.2);
            ctx.rectangle(0.0, 0.0, width as f64, height as f64);
            ctx.fill().unwrap();

            // Add subtle grid to help with positioning
            ctx.set_source_rgba(0.3, 0.3, 0.3, 0.3);
            ctx.set_line_width(1.0);

            // Draw grid lines every 50 pixels
            let mut x = 50.0;
            while x < width as f64 {
                ctx.move_to(x, 0.0);
                ctx.line_to(x, height as f64);
                x += 50.0;
            }

            let mut y = 50.0;
            while y < height as f64 {
                ctx.move_to(0.0, y);
                ctx.line_to(width as f64, y);
                y += 50.0;
            }
            ctx.stroke().unwrap();

            // Draw instruction text with background for visibility
            let instruction_text = "Current desktop view - Click and drag to select rectangle area • Press Escape to cancel";
            ctx.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            ctx.set_font_size(16.0);

            // Measure text width for proper background sizing
            let text_extents = ctx.text_extents(instruction_text).unwrap();
            let text_width = text_extents.width();
            let text_height = text_extents.height();

            // Draw background for text with rounded corners
            ctx.set_source_rgba(0.0, 0.0, 0.0, 0.8);
            ctx.rectangle(10.0, 10.0, text_width + 20.0, text_height + 15.0);
            ctx.fill().unwrap();

            // Draw the instruction text
            ctx.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            ctx.move_to(20.0, 30.0);
            ctx.show_text(instruction_text).unwrap();

            if let (Some(start), Some(end)) =
                (*selection_start_draw.borrow(), *selection_end_draw.borrow())
            {
                let x = start.0.min(end.0);
                let y = start.1.min(end.1);
                let w = (end.0 - start.0).abs();
                let h = (end.1 - start.1).abs();

                // Clear the selected area to show a brighter preview
                ctx.save().unwrap();
                ctx.rectangle(x, y, w, h);
                ctx.clip();

                // Redraw the preview pattern at full brightness for selected area
                ctx.scale(
                    width as f64 / screen_info.0 as f64,
                    height as f64 / screen_info.1 as f64,
                );
                ctx.set_source_surface(&preview_surface, 0.0, 0.0).unwrap();
                ctx.paint().unwrap();
                ctx.restore().unwrap();

                // Draw thick selection border with animated effect
                ctx.set_source_rgb(0.2, 0.6, 1.0); // Blue selection color
                ctx.set_line_width(3.0);
                ctx.rectangle(x, y, w, h);
                ctx.stroke().unwrap();

                // Add inner white border for better visibility
                ctx.set_source_rgb(1.0, 1.0, 1.0);
                ctx.set_line_width(1.0);
                ctx.rectangle(x + 1.5, y + 1.5, w - 3.0, h - 3.0);
                ctx.stroke().unwrap();

                // Draw corner handles to indicate interactive selection
                let handle_size = 8.0;
                ctx.set_source_rgb(0.2, 0.6, 1.0);
                // Top-left corner
                ctx.rectangle(
                    x - handle_size / 2.0,
                    y - handle_size / 2.0,
                    handle_size,
                    handle_size,
                );
                ctx.fill().unwrap();
                // Top-right corner
                ctx.rectangle(
                    x + w - handle_size / 2.0,
                    y - handle_size / 2.0,
                    handle_size,
                    handle_size,
                );
                ctx.fill().unwrap();
                // Bottom-left corner
                ctx.rectangle(
                    x - handle_size / 2.0,
                    y + h - handle_size / 2.0,
                    handle_size,
                    handle_size,
                );
                ctx.fill().unwrap();
                // Bottom-right corner
                ctx.rectangle(
                    x + w - handle_size / 2.0,
                    y + h - handle_size / 2.0,
                    handle_size,
                    handle_size,
                );
                ctx.fill().unwrap();

                // Draw dimension text with background
                let text = format!("{}×{}", w as i32, h as i32);
                ctx.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                ctx.set_font_size(16.0);

                let text_extents = ctx.text_extents(&text).unwrap();
                let text_x = x + 8.0;
                let text_y = y + 25.0;

                // Draw text background
                ctx.set_source_rgba(0.0, 0.0, 0.0, 0.8);
                ctx.rectangle(
                    text_x - 4.0,
                    text_y - text_extents.height() - 4.0,
                    text_extents.width() + 8.0,
                    text_extents.height() + 8.0,
                );
                ctx.fill().unwrap();

                // Draw text
                ctx.set_source_rgb(1.0, 1.0, 1.0);
                ctx.move_to(text_x, text_y);
                ctx.show_text(&text).unwrap();
            }
        });

        // Mouse event handling
        let gesture_click = gtk4::GestureClick::new();
        let selection_start_click = selection_start.clone();
        let selection_end_click = selection_end.clone();
        let is_selecting_click = is_selecting.clone();
        let drawing_area_click = drawing_area.clone();

        gesture_click.connect_pressed(move |_, _, x, y| {
            *selection_start_click.borrow_mut() = Some((x, y));
            *selection_end_click.borrow_mut() = Some((x, y));
            *is_selecting_click.borrow_mut() = true;
            drawing_area_click.queue_draw();
        });

        let selection_start_release = selection_start.clone();
        let selection_end_release = selection_end.clone();
        let is_selecting_release = is_selecting.clone();
        let overlay_window_release = overlay_window.clone();
        let app_release = app.clone();
        let parent_window_release = parent_window.clone();
        let original_png_data_release = original_png_data.clone();

        gesture_click.connect_released(move |_, _, x, y| {
            if *is_selecting_release.borrow() {
                *selection_end_release.borrow_mut() = Some((x, y));
                *is_selecting_release.borrow_mut() = false;

                // Get selection bounds
                if let (Some(start), Some(end)) = (
                    *selection_start_release.borrow(),
                    *selection_end_release.borrow(),
                ) {
                    let x = start.0.min(end.0) as i32;
                    let y = start.1.min(end.1) as i32;
                    let w = (end.0 - start.0).abs() as i32;
                    let h = (end.1 - start.1).abs() as i32;

                    if w > 10 && h > 10 {
                        // Minimum size check
                        overlay_window_release.close();

                        // Use the stored PNG data and crop it directly
                        if let Some(ref png_data) = original_png_data_release {
                            match crop_png_data_direct(png_data, x, y, w, h) {
                                Ok(cropped_png) => {
                                    proceed_with_cropped_screenshot(
                                        app_release.clone(),
                                        parent_window_release.clone(),
                                        cropped_png,
                                    );
                                }
                                Err(e) => {
                                    error!("Failed to crop PNG data: {}", e);
                                    // Fallback to taking a new screenshot
                                    let rect = Some((x, y, w, h));
                                    proceed_with_screenshot(
                                        app_release.clone(),
                                        parent_window_release.clone(),
                                        rect,
                                    );
                                }
                            }
                        } else {
                            error!("No PNG data available for cropping, falling back to new screenshot");
                            // Fallback to taking a new screenshot
                            let rect = Some((x, y, w, h));
                            proceed_with_screenshot(
                                app_release.clone(),
                                parent_window_release.clone(),
                                rect,
                            );
                        }
                    } else {
                        overlay_window_release.close();
                        parent_window_release.set_visible(true);
                    }
                }
            }
        });

        // Mouse motion for live selection
        let motion_controller = gtk4::EventControllerMotion::new();
        let selection_end_motion = selection_end.clone();
        let is_selecting_motion = is_selecting.clone();
        let drawing_area_motion = drawing_area.clone();

        motion_controller.connect_motion(move |_, x, y| {
            if *is_selecting_motion.borrow() {
                *selection_end_motion.borrow_mut() = Some((x, y));
                drawing_area_motion.queue_draw();
            }
        });

        // Keyboard handling (Escape to cancel)
        let key_controller = gtk4::EventControllerKey::new();
        let overlay_window_key = overlay_window.clone();
        let parent_window_key = parent_window.clone();

        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gdk4::Key::Escape {
                overlay_window_key.close();
                parent_window_key.set_visible(true);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });

        drawing_area.add_controller(gesture_click);
        drawing_area.add_controller(motion_controller);
        drawing_area.add_controller(key_controller);
        drawing_area.set_can_focus(true);

        overlay_window.set_child(Some(&drawing_area));

        overlay_window.present();
        gtk4::prelude::GtkWindowExt::set_focus(&overlay_window, Some(&drawing_area));

        glib::ControlFlow::Break
    });
}

fn proceed_with_screenshot(
    app: Application,
    window: ApplicationWindow,
    rect: Option<(i32, i32, i32, i32)>,
) {
    // Create a channel for communication between threads
    let (sender, receiver) = mpsc::channel();

    // Spawn a thread for screenshot capture
    thread::spawn(move || {
        info!("Screenshot capture thread started");

        // Add delay to ensure window is hidden
        thread::sleep(std::time::Duration::from_millis(500));
        info!("Starting screenshot capture after delay");

        let result = take_screenshot_sync(rect);
        match &result {
            Ok(_) => info!("Screenshot capture completed successfully"),
            Err(e) => error!("Screenshot capture failed: {}", e),
        }

        if let Err(e) = sender.send(result) {
            error!("Failed to send screenshot result: {}", e);
        }
    });

    // Use glib timeout to check for completion
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(result) => {
                match result {
                    Ok(image_data) => {
                        info!(
                            "Screenshot captured successfully ({} bytes), opening editor",
                            image_data.len()
                        );

                        // Close the capture window
                        window.close();

                        // Create and show the annotation editor
                        match AnnotationEditor::new(&app, image_data) {
                            Ok(editor) => {
                                info!("Editor created successfully");
                                editor.show();
                            }
                            Err(e) => {
                                error!("Failed to create editor: {}", e);
                                show_error_dialog(
                                    &window,
                                    &format!("Failed to open editor: {}", e),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to capture screenshot: {}", e);

                        // Show the window again and display error
                        window.set_visible(true);
                        show_error_dialog(&window, &format!("Failed to capture screenshot: {}", e));
                    }
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => {
                // Still waiting for screenshot
                glib::ControlFlow::Continue
            }
            Err(_) => {
                error!("Screenshot capture thread failed");
                window.set_visible(true);
                show_error_dialog(&window, "Screenshot capture failed unexpectedly");
                glib::ControlFlow::Break
            }
        }
    });
}

fn take_screenshot_sync(rect: Option<(i32, i32, i32, i32)>) -> Result<Vec<u8>> {
    info!("Initializing screenshot capture");
    let capture = ScreenshotCapture::new();

    info!("Attempting to capture screenshot");
    let result = if let Some((x, y, w, h)) = rect {
        info!("Capturing rectangle region: {}x{} at ({}, {})", w, h, x, y);
        capture.take_screenshot_region_blocking(x, y, w, h)
    } else {
        capture.take_screenshot_blocking()
    };

    match &result {
        Ok(data) => info!("Screenshot captured: {} bytes", data.len()),
        Err(e) => error!("Screenshot capture error: {}", e),
    }

    result
}

fn show_error_dialog(parent: &ApplicationWindow, message: &str) {
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .text("Screenshot Error")
        .secondary_text(message)
        .buttons(gtk4::ButtonsType::Ok)
        .build();

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.present();
}

fn capture_current_screen_for_preview_with_data(
    width: i32,
    height: i32,
) -> (cairo::ImageSurface, Option<Vec<u8>>) {
    info!("Attempting to capture current screen state for preview with original data");

    // Longer delay to ensure capture UI window is completely hidden
    std::thread::sleep(std::time::Duration::from_millis(500));

    let capture = ScreenshotCapture::new();

    match capture.take_screenshot_blocking() {
        Ok(png_data) => {
            info!("Successfully captured screen for preview");
            // Load PNG data into an image
            match image::load_from_memory(&png_data) {
                Ok(img) => {
                    // Convert to RGBA format
                    let rgba_img = img.to_rgba8();
                    let (img_width, img_height) = rgba_img.dimensions();
                    let pixels = rgba_img.into_raw();

                    // Convert RGBA to BGRA for Cairo (Cairo expects BGRA on little-endian systems)
                    let mut bgra_pixels = Vec::with_capacity(pixels.len());
                    for chunk in pixels.chunks(4) {
                        if chunk.len() == 4 {
                            bgra_pixels.push(chunk[2]); // B
                            bgra_pixels.push(chunk[1]); // G
                            bgra_pixels.push(chunk[0]); // R
                            bgra_pixels.push(chunk[3]); // A
                        }
                    }

                    // Create Cairo ImageSurface with actual screen capture
                    match cairo::ImageSurface::create_for_data(
                        bgra_pixels,
                        cairo::Format::ARgb32,
                        img_width as i32,
                        img_height as i32,
                        img_width as i32 * 4,
                    ) {
                        Ok(surface) => {
                            info!(
                                "Created Cairo surface from screen capture: {}x{}",
                                img_width, img_height
                            );
                            return (surface, Some(png_data));
                        }
                        Err(e) => {
                            log::warn!("Failed to create Cairo surface from capture: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to load captured image: {}", e);
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to capture screen for preview: {}", e);
        }
    }

    // Fallback to preview pattern if capture fails
    info!("Falling back to preview pattern");
    (create_screen_preview_pattern(width, height), None)
}

fn get_screen_info_without_capture() -> (i32, i32) {
    // Get screen dimensions using GDK without actually capturing
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

fn create_screen_preview_pattern(width: i32, height: i32) -> cairo::ImageSurface {
    // Create a visual pattern that represents the desktop without actually capturing it
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height)
        .expect("Failed to create preview surface");
    let ctx = cairo::Context::new(&surface).expect("Failed to create cairo context");

    // Create a gradient background that simulates a desktop
    let gradient = cairo::LinearGradient::new(0.0, 0.0, width as f64, height as f64);
    gradient.add_color_stop_rgba(0.0, 0.2, 0.3, 0.5, 1.0); // Blue-ish top
    gradient.add_color_stop_rgba(1.0, 0.1, 0.2, 0.4, 1.0); // Darker bottom

    ctx.set_source(&gradient).unwrap();
    ctx.rectangle(0.0, 0.0, width as f64, height as f64);
    ctx.fill().unwrap();

    // Add some visual elements to simulate a desktop
    // Taskbar simulation
    ctx.set_source_rgba(0.0, 0.0, 0.0, 0.8);
    ctx.rectangle(0.0, height as f64 - 48.0, width as f64, 48.0);
    ctx.fill().unwrap();

    // Simulate some desktop icons/windows
    let window_colors = [
        (0.9, 0.9, 0.9, 0.9), // Light window
        (0.8, 0.8, 0.9, 0.9), // Slightly blue window
        (0.9, 0.8, 0.8, 0.9), // Slightly red window
    ];

    for (i, &(r, g, b, a)) in window_colors.iter().enumerate() {
        let x = 50.0 + (i as f64 * 220.0);
        let y = 50.0 + (i as f64 * 80.0);
        let w = 200.0;
        let h = 150.0;

        // Window shadow
        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.3);
        ctx.rectangle(x + 5.0, y + 5.0, w, h);
        ctx.fill().unwrap();

        // Window
        ctx.set_source_rgba(r, g, b, a);
        ctx.rectangle(x, y, w, h);
        ctx.fill().unwrap();

        // Window title bar
        ctx.set_source_rgba(r * 0.8, g * 0.8, b * 0.8, a);
        ctx.rectangle(x, y, w, 30.0);
        ctx.fill().unwrap();
    }

    // Add text indicating this is a preview
    ctx.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    ctx.set_font_size(24.0);
    ctx.set_source_rgba(1.0, 1.0, 1.0, 0.7);

    let preview_text = "Desktop Preview - Screen capture unavailable";
    let text_extents = ctx.text_extents(preview_text).unwrap();
    let text_x = (width as f64 - text_extents.width()) / 2.0;
    let text_y = height as f64 / 2.0;

    // Text background
    ctx.set_source_rgba(0.0, 0.0, 0.0, 0.6);
    ctx.rectangle(
        text_x - 20.0,
        text_y - 30.0,
        text_extents.width() + 40.0,
        50.0,
    );
    ctx.fill().unwrap();

    // Text
    ctx.set_source_rgba(1.0, 1.0, 1.0, 0.9);
    ctx.move_to(text_x, text_y);
    ctx.show_text(preview_text).unwrap();

    surface
}

fn crop_png_data_direct(
    png_data: &[u8],
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<Vec<u8>> {
    info!(
        "Cropping PNG data directly: {}x{} at ({}, {})",
        width, height, x, y
    );

    // Load the image from PNG bytes
    let image = image::load_from_memory(png_data)
        .map_err(|e| anyhow::anyhow!("Failed to load image for cropping: {}", e))?;

    let (img_width, img_height) = image.dimensions();
    info!("Original image dimensions: {}x{}", img_width, img_height);

    // Validate crop bounds
    let crop_x = x.max(0) as u32;
    let crop_y = y.max(0) as u32;
    let crop_width = width.min(img_width as i32 - x).max(1) as u32;
    let crop_height = height.min(img_height as i32 - y).max(1) as u32;

    if crop_x >= img_width || crop_y >= img_height {
        return Err(anyhow::anyhow!("Crop region is outside image bounds"));
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
        .map_err(|e| anyhow::anyhow!("Failed to convert cropped image to PNG: {}", e))?;

    info!("Cropped image converted to PNG, {} bytes", buffer.len());
    Ok(buffer)
}

fn proceed_with_cropped_screenshot(app: Application, window: ApplicationWindow, png_data: Vec<u8>) {
    info!(
        "Opening editor with cropped screenshot ({} bytes)",
        png_data.len()
    );

    // Close the capture window
    window.close();

    // Create and show the annotation editor directly
    match AnnotationEditor::new(&app, png_data) {
        Ok(editor) => {
            info!("Editor created successfully");
            editor.show();
        }
        Err(e) => {
            error!("Failed to create editor: {}", e);
            show_error_dialog(&window, &format!("Failed to open editor: {}", e));
        }
    }
}

fn start_window_selection_capture(app: Application, parent_window: ApplicationWindow) {
    info!("Starting window selection capture");

    // Hide the parent window
    parent_window.set_visible(false);

    // Create window manager
    let window_manager = match window_manager::WindowManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            error!("Failed to create window manager: {}", e);
            show_error_dialog(&parent_window, &format!("Window selection not available: {}\n\nThis feature requires X11. On Wayland, please use Screen or Selection capture instead.", e));
            parent_window.set_visible(true);
            return;
        }
    };

    // Get list of windows
    let windows = match window_manager.list_windows() {
        Ok(windows) => windows,
        Err(e) => {
            error!("Failed to list windows: {}", e);
            show_error_dialog(&parent_window, &format!("Failed to enumerate windows: {}\n\nThis feature requires X11. On Wayland, please use Screen or Selection capture instead.", e));
            parent_window.set_visible(true);
            return;
        }
    };

    if windows.is_empty() {
        show_error_dialog(&parent_window, "No capturable windows found. Please ensure you have open windows that are not minimized.");
        parent_window.set_visible(true);
        return;
    }

    show_window_selection_dialog(app, parent_window, windows, window_manager);
}

fn show_window_selection_dialog(
    app: Application,
    parent_window: ApplicationWindow,
    windows: Vec<window_manager::WindowInfo>,
    window_manager: window_manager::WindowManager,
) {
    info!(
        "Showing window selection dialog with {} windows",
        windows.len()
    );

    // Create the window selection dialog
    let dialog = ApplicationWindow::builder()
        .application(&app)
        .title("Select Window to Capture")
        .default_width(500)
        .default_height(400)
        .resizable(true)
        .modal(true)
        .build();

    // Create main container
    let main_box = Box::new(Orientation::Vertical, 10);
    main_box.set_margin_start(20);
    main_box.set_margin_end(20);
    main_box.set_margin_top(20);
    main_box.set_margin_bottom(20);

    // Title label
    let title_label = Label::new(Some("Select a window to capture:"));
    title_label.add_css_class("title-2");
    title_label.set_margin_bottom(10);

    // Create scrolled window for the list
    let scrolled = ScrolledWindow::new();
    scrolled.set_policy(PolicyType::Never, PolicyType::Automatic);
    scrolled.set_vexpand(true);

    // Create list box for windows
    let list_box = ListBox::new();
    list_box.set_selection_mode(SelectionMode::Single);
    list_box.add_css_class("boxed-list");

    // Populate list with windows
    for window_info in &windows {
        let row_widget = create_window_list_row(window_info);
        let list_row = ListBoxRow::new();
        list_row.set_child(Some(&row_widget));
        list_row.set_activatable(true);
        list_row.set_selectable(true);
        list_box.append(&list_row);
    }

    scrolled.set_child(Some(&list_box));

    // Button container
    let button_box = Box::new(Orientation::Horizontal, 10);
    button_box.set_halign(gtk4::Align::End);
    button_box.set_margin_top(10);

    // Cancel button
    let cancel_button = Button::with_label("Cancel");
    let dialog_clone = dialog.clone();
    let parent_clone = parent_window.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_clone.close();
        parent_clone.set_visible(true);
    });

    // Capture button
    let capture_button = Button::with_label("Capture Window");
    capture_button.add_css_class("suggested-action");
    capture_button.set_sensitive(false); // Initially disabled

    // Create shared reference to window manager for all callbacks
    let window_manager_clone = std::rc::Rc::new(window_manager);

    // Enable capture button when selection changes
    let capture_button_clone = capture_button.clone();
    list_box.connect_row_selected(move |_, row| {
        let is_selected = row.is_some();
        capture_button_clone.set_sensitive(is_selected);
        if is_selected {
            info!("Window row selected, capture button enabled");
        }
    });

    // Add double-click activation for immediate capture
    let dialog_clone_activate = dialog.clone();
    let parent_clone_activate = parent_window.clone();
    let app_clone_activate = app.clone();
    let windows_clone_activate = windows.clone();
    let window_manager_activate = window_manager_clone.clone();

    list_box.connect_row_activated(move |_, activated_row| {
        let window_index = activated_row.index() as usize;
        if let Some(window_info) = windows_clone_activate.get(window_index) {
            info!(
                "Window row activated (double-clicked): {} (ID: {})",
                window_info.title, window_info.id
            );

            // Close the dialog
            dialog_clone_activate.close();

            // Start window capture immediately
            proceed_with_window_capture(
                app_clone_activate.clone(),
                parent_clone_activate.clone(),
                window_info.id,
                window_manager_activate.as_ref(),
            );
        }
    });

    // Handle capture button click
    let dialog_clone = dialog.clone();
    let parent_clone = parent_window.clone();
    let app_clone = app.clone();
    let list_box_clone = list_box.clone();
    let windows_clone = windows.clone();
    let window_manager_capture = window_manager_clone.clone();

    capture_button.connect_clicked(move |_| {
        if let Some(selected_row) = list_box_clone.selected_row() {
            let window_index = selected_row.index() as usize;
            if let Some(window_info) = windows_clone.get(window_index) {
                info!(
                    "Capturing window: {} (ID: {})",
                    window_info.title, window_info.id
                );

                // Close the dialog
                dialog_clone.close();

                // Start window capture
                proceed_with_window_capture(
                    app_clone.clone(),
                    parent_clone.clone(),
                    window_info.id,
                    window_manager_capture.as_ref(),
                );
            } else {
                error!("Failed to get window info for index: {}", window_index);
            }
        } else {
            error!("No window selected");
        }
    });

    // Add buttons to container
    button_box.append(&cancel_button);
    button_box.append(&capture_button);

    // Add all elements to main container
    main_box.append(&title_label);
    main_box.append(&scrolled);
    main_box.append(&button_box);

    dialog.set_child(Some(&main_box));

    // Handle window close
    let parent_clone = parent_window.clone();
    dialog.connect_close_request(move |_| {
        parent_clone.set_visible(true);
        glib::Propagation::Proceed
    });

    dialog.present();
}

fn create_window_list_row(window_info: &window_manager::WindowInfo) -> Box {
    let row_box = Box::new(Orientation::Horizontal, 12);
    row_box.set_margin_start(12);
    row_box.set_margin_end(12);
    row_box.set_margin_top(8);
    row_box.set_margin_bottom(8);

    // Window info container
    let info_box = Box::new(Orientation::Vertical, 4);
    info_box.set_hexpand(true);

    // Sanitize window title (remove null characters)
    let sanitized_title = window_info.title.replace('\0', "");
    let title_label = Label::new(Some(&sanitized_title));
    title_label.set_halign(gtk4::Align::Start);
    title_label.add_css_class("heading");
    title_label.set_ellipsize(pango::EllipsizeMode::End);
    title_label.set_max_width_chars(50);

    // Sanitize window class and create details
    let sanitized_class = window_info.class.replace('\0', "");
    let details = if !sanitized_class.is_empty() && sanitized_class != "Unknown" {
        format!(
            "{} • {}×{} • ID: {}",
            sanitized_class, window_info.width, window_info.height, window_info.id
        )
    } else {
        format!(
            "{}×{} • ID: {}",
            window_info.width, window_info.height, window_info.id
        )
    };

    let details_label = Label::new(Some(&details));
    details_label.set_halign(gtk4::Align::Start);
    details_label.add_css_class("dim-label");
    details_label.add_css_class("caption");
    details_label.set_ellipsize(pango::EllipsizeMode::End);

    info_box.append(&title_label);
    info_box.append(&details_label);

    // Add window icon placeholder
    let icon_box = Box::new(Orientation::Vertical, 0);
    icon_box.set_valign(gtk4::Align::Center);
    icon_box.set_size_request(32, 32);

    let icon_label = Label::new(Some("🪟"));
    icon_label.add_css_class("title-1");
    icon_box.append(&icon_label);

    row_box.append(&icon_box);
    row_box.append(&info_box);

    row_box
}

fn proceed_with_window_capture(
    app: Application,
    parent_window: ApplicationWindow,
    window_id: u64,
    window_manager: &window_manager::WindowManager,
) {
    info!(
        "Proceeding with window capture for window ID: {}",
        window_id
    );

    // Capture the window immediately - no need for async delay
    match window_manager.capture_window(window_id) {
        Ok(png_data) => {
            info!("Window captured successfully, {} bytes", png_data.len());

            // Close the parent window
            parent_window.close();

            // Open the editor with the captured window
            match AnnotationEditor::new(&app, png_data) {
                Ok(editor) => {
                    info!("Editor created successfully for window capture");
                    editor.show();
                }
                Err(e) => {
                    error!("Failed to create editor: {}", e);
                    show_error_dialog(&parent_window, &format!("Failed to open editor: {}", e));
                    parent_window.set_visible(true);
                }
            }
        }
        Err(e) => {
            error!("Failed to capture window: {}", e);
            show_error_dialog(&parent_window, &format!("Failed to capture window: {}\n\nTip: Make sure the target window is visible and not minimized.", e));
            parent_window.set_visible(true);
        }
    }
}
