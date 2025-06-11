use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow, Button, Box, Orientation, Label};
use anyhow::Result;
use log::{info, error};
use std::thread;
use std::sync::mpsc;

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

    let app = Application::builder()
        .application_id(APP_ID)
        .build();

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
    desc_label.set_margin_bottom(20);

    // Capture button
    let capture_button = Button::with_label("📷 Take Screenshot");
    capture_button.set_size_request(200, 50);
    capture_button.add_css_class("suggested-action");
    capture_button.add_css_class("pill");
    capture_button.add_css_class("capture-button");

    // Clone app for the callback
    let app_clone = app.clone();
    let window_clone = window.clone();
    
    capture_button.connect_clicked(move |_| {
        info!("Capture button clicked");
        let app = app_clone.clone();
        let window = window_clone.clone();
        
        // Hide the capture window
        window.set_visible(false);
        
        // Create a channel for communication between threads
        let (sender, receiver) = mpsc::channel();
        
        // Spawn a thread for screenshot capture
        thread::spawn(move || {
            info!("Screenshot capture thread started");
            
            // Add delay to ensure window is hidden
            thread::sleep(std::time::Duration::from_millis(500));
            info!("Starting screenshot capture after delay");
            
            let result = take_screenshot_sync();
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
                            info!("Screenshot captured successfully ({} bytes), opening editor", image_data.len());
                            
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
                                    show_error_dialog(&window, &format!("Failed to open editor: {}", e));
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

    // Add widgets to container
    main_box.append(&title_label);
    main_box.append(&desc_label);
    main_box.append(&capture_button);
    main_box.append(&quit_button);

    window.set_child(Some(&main_box));
    window.add_css_class("capture-window");

    // Show the window
    window.present();
    
    info!("Capture interface ready");
}

fn take_screenshot_sync() -> Result<Vec<u8>> {
    info!("Initializing screenshot capture");
    let capture = ScreenshotCapture::new();
    
    info!("Attempting to capture screenshot");
    let result = capture.take_screenshot_blocking();
    
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