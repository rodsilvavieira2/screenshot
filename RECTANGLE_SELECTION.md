# Rectangle Selection Feature

This document explains the new rectangle selection feature in Flint that allows users to capture specific rectangular areas of their screen.

## Overview

The rectangle selection feature provides a visual overlay interface that lets users click and drag to select a specific rectangular region for screenshot capture, instead of capturing the entire screen.

## How to Use

### Basic Workflow

1. **Launch Flint**: Run `flint` or `cargo run`
2. **Choose Rectangle Mode**: Click the "🔲 Select Rectangle Area" button
3. **Select Region**: 
   - A semi-transparent overlay appears covering your entire screen
   - Click and drag to draw a rectangle around the area you want to capture
   - The selected area will be highlighted with a red border
4. **Confirm Selection**: Release the mouse button to capture the selected region
5. **Edit**: The annotation editor opens with your cropped screenshot

### Controls

#### Mouse Controls:
- **Click + Drag**: Start selection and resize the rectangle
- **Release**: Confirm selection and take screenshot

#### Keyboard Controls:
- **Escape**: Cancel selection and return to main interface

### Visual Feedback

- **Dark Overlay**: Semi-transparent dark layer covers the entire screen
- **Clear Rectangle**: Selected area is transparent, showing the underlying screen
- **Red Border**: 2px red outline shows the exact selection bounds
- **Live Preview**: Rectangle updates in real-time as you drag

## Technical Implementation

### Architecture

```
User Input → Overlay Window → Region Selection → Image Cropping → Editor
```

### Key Components

#### 1. Overlay Window
```rust
// Creates fullscreen transparent window
let overlay_window = ApplicationWindow::builder()
    .application(&app)
    .title("Select Rectangle Area")
    .decorated(false)  // No window decorations
    .build();

overlay_window.fullscreen();
overlay_window.set_opacity(0.3);  // Semi-transparent
```

#### 2. Selection Drawing
```rust
// Cairo drawing for selection rectangle
ctx.set_source_rgba(0.0, 0.0, 0.0, 0.5);  // Dark overlay
ctx.rectangle(0.0, 0.0, width as f64, height as f64);
ctx.fill().unwrap();

// Clear selection area
ctx.set_operator(cairo::Operator::Clear);
ctx.rectangle(x, y, w, h);
ctx.fill().unwrap();

// Red selection border
ctx.set_source_rgb(1.0, 0.0, 0.0);
ctx.set_line_width(2.0);
ctx.rectangle(x, y, w, h);
ctx.stroke().unwrap();
```

#### 3. Image Cropping
```rust
pub fn take_screenshot_region_blocking(&self, x: i32, y: i32, width: i32, height: i32) -> Result<Vec<u8>> {
    // 1. Capture full screen
    let full_screenshot = self.take_screenshot_blocking()?;
    
    // 2. Crop to selected region
    self.crop_image_region(&full_screenshot, x, y, width, height)
}
```

### Event Handling

#### Mouse Events:
- **Pressed**: Start selection, record initial coordinates
- **Motion**: Update selection end coordinates, redraw rectangle
- **Released**: Finalize selection, validate size, proceed with capture

#### State Management:
```rust
let selection_start = Rc<new(RefCell::new(None::<(f64, f64)>));
let selection_end = Rc::new(RefCell::new(None::<(f64, f64)>));
let is_selecting = Rc::new(RefCell::new(false));
```

## Features

### Smart Validation
- **Minimum Size**: Selections must be at least 10x10 pixels
- **Boundary Checking**: Prevents selection outside screen bounds
- **Coordinate Adjustment**: Automatically adjusts for negative dimensions

### Error Handling
- **Invalid Selections**: Too small rectangles are ignored
- **Out of Bounds**: Selections are clipped to screen dimensions
- **Cancel Support**: Escape key cancels selection gracefully

### Performance Optimizations
- **Real-time Rendering**: Smooth rectangle updates during dragging
- **Efficient Cropping**: Uses image library's optimized crop function
- **Memory Management**: Proper cleanup of overlay resources

## User Experience

### Visual Design
- **Intuitive Interface**: Clear visual feedback for selection area
- **Professional Appearance**: Semi-transparent overlay maintains context
- **Responsive Feel**: Immediate visual response to mouse movement

### Accessibility
- **Keyboard Support**: Escape key for cancellation
- **Clear Instructions**: Helpful text in main interface
- **Error Recovery**: Easy return to main interface on invalid selections

## Comparison with Full Screenshot

| Feature | Full Screenshot | Rectangle Selection |
|---------|----------------|-------------------|
| **Speed** | Instant | ~2-3 seconds |
| **Precision** | Entire screen | Exact user selection |
| **File Size** | Large | Smaller (cropped) |
| **Use Case** | General capture | Focused content |
| **Interaction** | One click | Click + drag |

## Troubleshooting

### Common Issues

#### Selection Not Working
- **Cause**: Mouse events not registering
- **Solution**: Ensure drawing area has focus, try clicking again

#### Rectangle Not Visible
- **Cause**: Graphics rendering issues
- **Solution**: Check Cairo/GTK4 installation, try full screenshot mode

#### Cropped Image Empty
- **Cause**: Selection coordinates invalid
- **Solution**: Ensure minimum 10x10 pixel selection

#### Overlay Stuck
- **Cause**: Window focus issues
- **Solution**: Press Escape or Alt+Tab to other windows

### Debug Information

```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Look for rectangle selection logs
grep -i "rectangle\|region\|crop" debug.log
```

## Limitations

### Current Version (V1.0)
- **X11 Only**: Rectangle selection works via screen capture + crop
- **Single Monitor**: Only works on primary display
- **No Portal Integration**: Doesn't use Wayland portal region selection

### Future Enhancements
- **Native Portal Support**: Direct region selection via xdg-desktop-portal
- **Multi-Monitor**: Support for selections across multiple displays
- **Shape Selection**: Non-rectangular selection tools
- **Preview Mode**: Live preview of selected content

## Integration with Annotation Tools

### Seamless Workflow
1. **Region Selection**: User selects specific area
2. **Automatic Editor**: Opens with cropped screenshot loaded
3. **Full Tool Access**: All annotation tools work on cropped image
4. **Standard Export**: Save/copy functions work normally

### Performance Benefits
- **Smaller Images**: Faster rendering and editing
- **Focused Annotation**: Easier to annotate specific content
- **Reduced File Sizes**: More efficient storage and sharing

## Code Examples

### Basic Usage (Application Code)
```rust
// In main interface
rect_button.connect_clicked(move |_| {
    start_screenshot_capture(app.clone(), window.clone(), true);
});

// Rectangle mode enabled
if is_rectangle {
    show_rectangle_selection(app, window);
} else {
    proceed_with_screenshot(app, window, None);
}
```

### Custom Rectangle Selection (Advanced)
```rust
// For custom applications integrating Flint's capture
let capture = ScreenshotCapture::new();
let image_data = capture.take_screenshot_region_blocking(100, 100, 800, 600)?;
```

## Best Practices

### For Users
1. **Clear Selections**: Make sure your rectangle clearly encompasses desired content
2. **Minimum Size**: Select at least 10x10 pixels for valid capture
3. **Single Drag**: Complete selection in one continuous drag motion
4. **Cancel When Needed**: Use Escape if you need to start over

### For Developers
1. **Validate Coordinates**: Always check selection bounds before cropping
2. **Handle Edge Cases**: Account for window decorations and multiple monitors
3. **Provide Feedback**: Clear visual and status feedback for user actions
4. **Optimize Performance**: Minimize overlay rendering overhead

This rectangle selection feature significantly enhances Flint's usability by allowing precise content capture while maintaining the simple, fast workflow that users expect.