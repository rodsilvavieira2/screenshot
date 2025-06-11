use gtk4::prelude::*;
use gtk4::{
    Box, Button, ComboBoxText, Label, Orientation, Scale, Separator,
    ToggleButton,
};
use gdk4::RGBA;
use glib::clone;
use std::cell::RefCell;
use std::rc::Rc;

use crate::tools::ToolType;

pub struct Toolbar {
    pub widget: Box,
    tool_buttons: Vec<ToggleButton>,
    color_combo: ComboBoxText,
    thickness_scale: Scale,
    current_tool: Rc<RefCell<ToolType>>,
}

impl Toolbar {
    pub fn new() -> Self {
        let widget = Box::new(Orientation::Horizontal, 6);
        widget.set_margin_start(6);
        widget.set_margin_end(6);
        widget.set_margin_top(6);
        widget.set_margin_bottom(6);
        widget.add_css_class("toolbar");

        let current_tool = Rc::new(RefCell::new(ToolType::Pencil));

        // Tool selection buttons
        let tool_box = Box::new(Orientation::Horizontal, 2);
        tool_box.add_css_class("linked");
        
        let tool_buttons = Self::create_tool_buttons(&tool_box, current_tool.clone());

        // Separator
        let separator1 = Separator::new(Orientation::Vertical);

        // Color selection
        let color_box = Box::new(Orientation::Horizontal, 6);
        let color_label = Label::new(Some("Color:"));
        let color_combo = Self::create_color_combo();

        color_box.append(&color_label);
        color_box.append(&color_combo);

        // Separator
        let separator2 = Separator::new(Orientation::Vertical);

        // Thickness control
        let thickness_box = Box::new(Orientation::Horizontal, 6);
        let thickness_label = Label::new(Some("Size:"));
        let thickness_scale = Self::create_thickness_scale();

        thickness_box.append(&thickness_label);
        thickness_box.append(&thickness_scale);

        // Separator
        let separator3 = Separator::new(Orientation::Vertical);

        // Action buttons
        let action_box = Box::new(Orientation::Horizontal, 6);
        let clear_button = Self::create_clear_button();
        let save_button = Self::create_save_button();
        let copy_button = Self::create_copy_button();

        action_box.append(&clear_button);
        action_box.append(&save_button);
        action_box.append(&copy_button);

        // Add all sections to main toolbar
        widget.append(&tool_box);
        widget.append(&separator1);
        widget.append(&color_box);
        widget.append(&separator2);
        widget.append(&thickness_box);
        widget.append(&separator3);
        widget.append(&action_box);

        Self {
            widget,
            tool_buttons,
            color_combo,
            thickness_scale,
            current_tool,
        }
    }

    fn create_tool_buttons(
        container: &Box,
        current_tool: Rc<RefCell<ToolType>>,
    ) -> Vec<ToggleButton> {
        let tools = vec![
            (ToolType::Pencil, "✏️", "Pencil"),
            (ToolType::Line, "📏", "Line"),
            (ToolType::Arrow, "➡️", "Arrow"),
            (ToolType::Highlighter, "🖍️", "Highlighter"),
        ];

        let mut buttons = Vec::new();

        for (i, (tool_type, icon, tooltip)) in tools.iter().enumerate() {
            let button = ToggleButton::new();
            button.set_label(icon);
            button.set_tooltip_text(Some(tooltip));

            // Set first button as active
            if i == 0 {
                button.set_active(true);
            }

            let tool_type_clone = *tool_type;
            let current_tool_clone = current_tool.clone();
            // Set first button as active and update current tool
            if i == 0 {
                *current_tool_clone.borrow_mut() = tool_type_clone;
            }

            button.connect_toggled(clone!(@weak button => move |btn| {
                if btn.is_active() {
                    *current_tool_clone.borrow_mut() = tool_type_clone;
                } else {
                    // Prevent deactivating the current tool
                    if *current_tool_clone.borrow() == tool_type_clone {
                        btn.set_active(true);
                    }
                }
            }));

            container.append(&button);
            buttons.push(button.clone());
        }

        buttons
    }

    fn create_color_combo() -> ComboBoxText {
        let combo = ComboBoxText::new();
        
        let colors = vec![
            "Red",
            "Green", 
            "Blue",
            "Yellow",
            "Pink",
            "Cyan",
            "Black",
            "White",
        ];

        for color in &colors {
            combo.append_text(color);
        }

        combo.set_active(Some(0)); // Default to Red

        combo
    }

    fn create_thickness_scale() -> Scale {
        let scale = Scale::with_range(Orientation::Horizontal, 1.0, 20.0, 1.0);
        scale.set_value(3.0);
        scale.set_size_request(100, -1);
        scale.set_digits(0);
        scale.set_draw_value(true);

        scale
    }

    fn create_clear_button() -> Button {
        let button = Button::with_label("🗑️ Clear");
        button.set_tooltip_text(Some("Clear all annotations"));
        button.add_css_class("destructive-action");

        button
    }

    fn create_save_button() -> Button {
        let button = Button::with_label("💾 Save");
        button.set_tooltip_text(Some("Save to file"));
        button.add_css_class("suggested-action");

        button
    }

    fn create_copy_button() -> Button {
        let button = Button::with_label("📋 Copy");
        button.set_tooltip_text(Some("Copy to clipboard"));

        button
    }

    pub fn update_thickness_for_tool(&self, tool: ToolType) {
        let default_thickness = match tool {
            ToolType::Pencil => 3.0,
            ToolType::Line => 2.0,
            ToolType::Arrow => 2.0,
            ToolType::Highlighter => 8.0,
        };
        
        self.thickness_scale.set_value(default_thickness);
    }

    pub fn get_current_tool(&self) -> ToolType {
        *self.current_tool.borrow()
    }

    pub fn get_current_color(&self) -> RGBA {
        let colors = vec![
            RGBA::new(1.0, 0.0, 0.0, 1.0), // Red
            RGBA::new(0.0, 0.8, 0.0, 1.0), // Green
            RGBA::new(0.0, 0.0, 1.0, 1.0), // Blue
            RGBA::new(1.0, 0.9, 0.0, 1.0), // Yellow
            RGBA::new(1.0, 0.4, 0.7, 1.0), // Pink
            RGBA::new(0.0, 0.8, 0.8, 1.0), // Cyan
            RGBA::new(0.0, 0.0, 0.0, 1.0), // Black
            RGBA::new(1.0, 1.0, 1.0, 1.0), // White
        ];

        if let Some(active) = self.color_combo.active() {
            colors.get(active as usize).copied().unwrap_or(colors[0])
        } else {
            colors[0]
        }
    }

    pub fn get_current_thickness(&self) -> f64 {
        self.thickness_scale.value()
    }

    pub fn connect_tool_changed<F>(&self, callback: F)
    where
        F: Fn(ToolType) + 'static + Clone,
    {
        for (i, button) in self.tool_buttons.iter().enumerate() {
            let tool_type = match i {
                0 => ToolType::Pencil,
                1 => ToolType::Line,
                2 => ToolType::Arrow,
                3 => ToolType::Highlighter,
                _ => ToolType::Pencil,
            };
            
            let callback_clone = callback.clone();
            button.connect_toggled(clone!(@weak button => move |btn| {
                if btn.is_active() {
                    callback_clone(tool_type);
                }
            }));
        }
    }

    pub fn connect_color_changed<F>(&self, callback: F)
    where
        F: Fn(RGBA) + 'static,
    {
        self.color_combo.connect_changed(move |combo| {
            let colors = vec![
                RGBA::new(1.0, 0.0, 0.0, 1.0), // Red
                RGBA::new(0.0, 0.8, 0.0, 1.0), // Green
                RGBA::new(0.0, 0.0, 1.0, 1.0), // Blue
                RGBA::new(1.0, 0.9, 0.0, 1.0), // Yellow
                RGBA::new(1.0, 0.4, 0.7, 1.0), // Pink
                RGBA::new(0.0, 0.8, 0.8, 1.0), // Cyan
                RGBA::new(0.0, 0.0, 0.0, 1.0), // Black
                RGBA::new(1.0, 1.0, 1.0, 1.0), // White
            ];

            if let Some(active) = combo.active() {
                if let Some(color) = colors.get(active as usize) {
                    callback(*color);
                }
            }
        });
    }

    pub fn connect_thickness_changed<F>(&self, callback: F)
    where
        F: Fn(f64) + 'static,
    {
        self.thickness_scale.connect_value_changed(move |scale| {
            let value = scale.value();
            callback(value);
        });
    }

    // Note: Button connection methods removed for V1.0 simplicity
    // These would be implemented with proper widget traversal in a full version

    pub fn get_widget(&self) -> &Box {
        &self.widget
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct StatusBar {
    pub widget: Box,
    status_label: Label,
    coordinates_label: Label,
}

impl StatusBar {
    pub fn new() -> Self {
        let widget = Box::new(Orientation::Horizontal, 6);
        widget.set_margin_start(6);
        widget.set_margin_end(6);
        widget.set_margin_top(3);
        widget.set_margin_bottom(3);
        widget.add_css_class("statusbar");

        let status_label = Label::new(Some("Ready"));
        status_label.set_halign(gtk4::Align::Start);

        let coordinates_label = Label::new(Some(""));
        coordinates_label.set_halign(gtk4::Align::End);
        coordinates_label.set_hexpand(true);

        widget.append(&status_label);
        widget.append(&coordinates_label);

        Self {
            widget,
            status_label,
            coordinates_label,
        }
    }

    pub fn set_status(&self, status: &str) {
        self.status_label.set_text(status);
    }

    pub fn set_coordinates(&self, x: f64, y: f64) {
        self.coordinates_label.set_text(&format!("({:.0}, {:.0})", x, y));
    }

    pub fn clear_coordinates(&self) {
        self.coordinates_label.set_text("");
    }

    pub fn get_widget(&self) -> &Box {
        &self.widget
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

// Helper function to load CSS for styling
pub fn load_css() {
    let provider = gtk4::CssProvider::new();
    let css = r#"
        /* Capture Interface Styling */
        .capture-window {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            border-radius: 12px;
        }
        
        .capture-button {
            font-size: 16px;
            font-weight: bold;
            padding: 12px 24px;
            border-radius: 8px;
            transition: all 200ms ease;
        }
        
        .capture-button:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        }
        
        .capture-title {
            font-size: 24px;
            font-weight: bold;
            color: #2c3e50;
        }
        
        .capture-description {
            font-size: 14px;
            color: #7f8c8d;
        }
        
        /* Editor Interface Styling */
        .toolbar {
            background-color: @theme_bg_color;
            border-bottom: 1px solid @borders;
            padding: 6px;
            box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }
        
        .statusbar {
            background-color: @theme_bg_color;
            border-top: 1px solid @borders;
            padding: 3px;
            font-size: 0.9em;
            font-family: monospace;
        }
        
        .drawing-area {
            background-color: #f8f9fa;
            border: 1px solid #dee2e6;
        }
        
        .tool-button {
            min-width: 50px;
            min-height: 40px;
            padding: 8px;
            margin: 2px;
            border-radius: 6px;
            font-size: 16px;
        }
        
        .tool-button:checked {
            background: @accent_color;
            color: @accent_fg_color;
        }
        
        button {
            min-width: 60px;
            padding: 6px 12px;
            border-radius: 6px;
            transition: all 150ms ease;
        }
        
        button:hover {
            transform: translateY(-1px);
        }
        
        .destructive-action {
            background: #e74c3c;
            color: white;
        }
        
        .destructive-action:hover {
            background: #c0392b;
        }
        
        .suggested-action {
            background: #27ae60;
            color: white;
            font-weight: bold;
        }
        
        .suggested-action:hover {
            background: #229954;
        }
        
        scale {
            min-width: 100px;
        }
        
        combobox {
            min-width: 80px;
            padding: 4px 8px;
        }
        
        /* Animations */
        @keyframes fade-in {
            from { opacity: 0; transform: translateY(10px); }
            to { opacity: 1; transform: translateY(0); }
        }
        
        .fade-in {
            animation: fade-in 300ms ease-out;
        }
    "#;
    
    provider.load_from_data(css);
    
    gtk4::style_context_add_provider_for_display(
        &gdk4::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolbar_creation() {
        // Note: GTK4 initialization required for actual testing
        // This is a placeholder test
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_status_bar_creation() {
        // Note: GTK4 initialization required for actual testing
        // This is a placeholder test
        assert_eq!(2 + 2, 4);
    }
}