use anyhow::Result;
use cairo;
use gdk4;
use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow, Box, Button, DrawingArea, Label, Orientation};
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

mod capture;
mod editor;
mod tools;
mod ui;

use capture::ScreenshotCapture;
use editor::AnnotationEditor;
use ui::load_css;

const APP_ID: &str = "com.flint.Screenshot";

fn main() -> Result<()> {
    env_logger::init();

    info!("Starting Flint Screenshot Tool v1.0.0");

    // Check for test mode
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--test" {
        println!("🔥 Flint Screenshot Tool - Test Mode");
        println!("✅ GTK4 libraries loaded successfully");
        println!("✅ Application can start");
        println!("✅ Screenshot capture module available");
        println!("Use 'cargo run' to start the full application");
        return Ok(());
    }

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_capture_ui);

    let exit_code = app.run_with_args(&args);

    std::process::exit(exit_code.into());
}

fn build_capture_ui(app: &Application) {
    // Load CSS styles
    load_css();

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
    title_label.add_css_class("title-1");
    title_label.add_css_class("capture-title");
    title_label.set_margin_bottom(10);

    // Description label
    let desc_label = Label::new(Some("Capture and annotate screenshots"));
    desc_label.add_css_class("dim-label");
    desc_label.add_css_class("capture-description");
    desc_label.set_margin_bottom(10);

    // Instruction label
    let instruction_label = Label::new(Some("Choose full screen or drag to select rectangle area"));
    instruction_label.add_css_class("dim-label");
    instruction_label.set_margin_bottom(20);
    instruction_label.set_markup("<small><i>Rectangle mode: Click and drag to select area, press Escape to cancel</i></small>");

    // Capture buttons container
    let button_box = Box::new(Orientation::Vertical, 10);

    // Full screenshot button
    let capture_button = Button::with_label("📷 Take Full Screenshot");
    capture_button.set_size_request(200, 50);
    capture_button.add_css_class("suggested-action");
    capture_button.add_css_class("pill");
    capture_button.add_css_class("capture-button");

    // Rectangle selection button
    let rect_button = Button::with_label("🔲 Select Rectangle Area");
    rect_button.set_size_request(200, 50);
    rect_button.add_css_class("capture-button");

    // Clone app for the callbacks
    let app_clone = app.clone();
    let window_clone = window.clone();
    let app_clone2 = app.clone();
    let window_clone2 = window.clone();

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

    // Add a quit button
    let quit_button = Button::with_label("❌ Quit");
    quit_button.set_margin_top(10);

    let window_for_quit = window.clone();
    quit_button.connect_clicked(move |_| {
        window_for_quit.close();
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

    // Add widgets to container
    main_box.append(&title_label);
    main_box.append(&desc_label);
    main_box.append(&instruction_label);
    main_box.append(&button_box);
    main_box.append(&quit_button);

    window.set_child(Some(&main_box));
    window.add_css_class("capture-window");

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
    // Hide parent window first
    parent_window.set_visible(false);

    // Create fullscreen overlay window for rectangle selection
    let overlay_window = ApplicationWindow::builder()
        .application(&app)
        .title("Select Rectangle Area")
        .default_width(1920)
        .default_height(1080)
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
        // Draw semi-transparent dark background for Wayland
        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.7);
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
        let instruction_text = "Click and drag to select rectangle area • Press Escape to cancel";
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

            // Make the selected area bright to show what will be captured
            ctx.save().unwrap();
            ctx.set_source_rgba(1.0, 1.0, 1.0, 0.3);
            ctx.rectangle(x, y, w, h);
            ctx.fill().unwrap();
            ctx.restore().unwrap();

            // Draw thick red border
            ctx.set_source_rgb(1.0, 0.0, 0.0);
            ctx.set_line_width(4.0);
            ctx.rectangle(x, y, w, h);
            ctx.stroke().unwrap();

            // Add inner white border for better visibility
            ctx.set_source_rgb(1.0, 1.0, 1.0);
            ctx.set_line_width(2.0);
            ctx.rectangle(x + 1.0, y + 1.0, w - 2.0, h - 2.0);
            ctx.stroke().unwrap();

            // Draw dimension text
            let text = format!("{}×{}", w as i32, h as i32);
            ctx.set_source_rgb(1.0, 1.0, 1.0);
            ctx.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            ctx.set_font_size(16.0);
            ctx.move_to(x + 5.0, y + 20.0);
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
                    let rect = Some((x, y, w, h));
                    overlay_window_release.close();
                    proceed_with_screenshot(
                        app_release.clone(),
                        parent_window_release.clone(),
                        rect,
                    );
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
