use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow};
use anyhow::Result;
use log::{info, error};

mod capture;
mod editor;
mod tools;
mod ui;

use capture::ScreenshotCapture;
use editor::AnnotationEditor;

const APP_ID: &str = "com.flint.Screenshot";

fn main() -> Result<()> {
    env_logger::init();
    
    info!("Starting Flint Screenshot Tool v1.0.0");

    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    
    let args: Vec<String> = std::env::args().collect();
    let exit_code = app.run_with_args(&args);
    
    std::process::exit(exit_code.into());
}

fn build_ui(app: &Application) {
    // Initialize screenshot capture in a separate thread to avoid blocking UI
    let app_clone = app.clone();
    
    glib::spawn_future_local(async move {
        match capture_screenshot().await {
            Ok(image_data) => {
                info!("Screenshot captured successfully");
                
                // Create and show the annotation editor
                let editor = AnnotationEditor::new(&app_clone, image_data);
                editor.show();
            }
            Err(e) => {
                error!("Failed to capture screenshot: {}", e);
                show_error_dialog(&app_clone, &format!("Failed to capture screenshot: {}", e));
            }
        }
    });
}

async fn capture_screenshot() -> Result<Vec<u8>> {
    let capture = ScreenshotCapture::new();
    capture.take_screenshot().await
}

fn show_error_dialog(app: &Application, message: &str) {
    let window = ApplicationWindow::new(app);
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(&window)
        .modal(true)
        .text("Error")
        .secondary_text(message)
        .buttons(gtk4::ButtonsType::Ok)
        .build();

    dialog.connect_response(|dialog, _| {
        dialog.close();
        std::process::exit(1);
    });

    dialog.present();
}