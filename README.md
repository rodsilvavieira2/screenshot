# Flint - High-Performance Screenshot & Annotation Tool

A fast, native screenshot and annotation utility for Linux desktop environments, built with Rust and GTK4.

## Features

- **Fast Screenshot Capture**: Sub-2-second workflow from capture to annotation
- **Modern Desktop Integration**: Native Wayland support via xdg-desktop-portal with X11 fallback
- **Essential Annotation Tools**:
  - ✏️ Pencil tool for freehand drawing
  - 📏 Line tool for straight lines
  - ➡️ Arrow tool with arrowheads
  - 🖍️ Highlighter with transparency
- **Export Options**: Save to file or copy to clipboard
- **Lightweight**: Under 50MB memory footprint during active use

## Requirements

### System Dependencies

On **Ubuntu/Debian**:
```bash
sudo apt update
sudo apt install build-essential pkg-config libgtk-4-dev libcairo2-dev libglib2.0-dev
```

On **Fedora/RHEL**:
```bash
sudo dnf install gcc pkg-config gtk4-devel cairo-devel glib2-devel
```

On **Arch Linux**:
```bash
sudo pacman -S base-devel pkg-config gtk4 cairo glib2
```

### Rust

Install Rust via [rustup](https://rustup.rs/):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

## Installation

### Build from Source

1. Clone the repository:
```bash
git clone <repository-url>
cd screenshot
```

2. Build the project:
```bash
cargo build --release
```

3. Run the application:
```bash
cargo run --release
```

Or install system-wide:
```bash
cargo install --path .
```

## Usage

### Basic Workflow

1. **Launch Flint**: Run `flint` from terminal or application menu
2. **Select Capture Area**: On Wayland, the system portal will appear for region/window selection
3. **Annotate**: Use the toolbar to select tools and draw on your screenshot
4. **Export**: Save to file or copy to clipboard

### Keyboard Shortcuts

- `Escape`: Cancel current drawing stroke
- `Ctrl+Z`: Undo last action (planned for future versions)

### Annotation Tools

#### Pencil Tool (✏️)
- Freehand drawing with adjustable thickness
- Perfect for circling areas or making notes

#### Line Tool (📏)
- Draw straight lines between two points
- Ideal for underlining or creating connections

#### Arrow Tool (➡️)
- Lines with arrowheads pointing to specific areas
- Great for highlighting bugs or important elements

#### Highlighter Tool (🖍️)
- Semi-transparent thick strokes
- Available in yellow, green, and pink

### Customization

- **Colors**: Choose from 8 predefined colors including red, green, blue, yellow, pink, cyan, black, and white
- **Thickness**: Adjustable from 1px to 20px
- **Tool-specific defaults**: Each tool has optimized default settings

## Desktop Integration

### Wayland (Recommended)
Flint uses the xdg-desktop-portal for secure, native screenshot capture on Wayland. This provides:
- Consistent UI with your desktop environment
- Secure permission handling
- Support for region, window, and full-screen capture

### X11 (Fallback)
On X11 systems, Flint falls back to direct screen capture. Note that V1.0 only supports full-screen capture in X11 mode.

## Troubleshooting

### Common Issues

**"GTK4 not found" error**:
- Ensure GTK4 development packages are installed
- Check that `pkg-config` can find GTK4: `pkg-config --cflags gtk4`

**Screenshot fails on Wayland**:
- Ensure xdg-desktop-portal is running: `systemctl --user status xdg-desktop-portal`
- Some desktop environments may require additional portal backends

**Slow performance**:
- Check system resources - Flint should use <50MB RAM
- Ensure hardware acceleration is available for GTK4

### Getting Help

1. Check the system dependencies are properly installed
2. Verify your desktop environment supports xdg-desktop-portal (for Wayland)
3. Look at the console output for error messages when running from terminal

## Development

### Project Structure

```
src/
├── main.rs           # Application entry point
├── capture.rs        # Screenshot capture (portal + X11 fallback)
├── editor.rs         # Main annotation editor window
├── tools.rs          # Drawing tools and stroke management
└── ui.rs            # UI components (toolbar, status bar)
```

### Building for Development

```bash
# Debug build with logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Check code quality
cargo clippy
cargo fmt
```

### Contributing

This project follows the PRD specifications for V1.0. Key areas for contribution:
- Performance optimizations
- Additional export formats
- Accessibility improvements
- Platform-specific enhancements

## Technical Details

- **Language**: Rust 2021 Edition
- **GUI Framework**: GTK4 with Cairo for drawing
- **Screenshot**: ashpd (Wayland portal) + screenshots crate (X11 fallback)
- **Image Processing**: image crate for format conversion
- **Async Runtime**: Tokio for portal communication

## License

MIT License - See LICENSE file for details.

## Version History

### V1.0.0 (Current)
- Initial release with core annotation features
- Wayland + X11 support
- PNG export and clipboard functionality
- Essential drawing tools (pencil, line, arrow, highlighter)

### Planned Features (Future Versions)
- Multi-level undo/redo
- Text annotations
- Shape tools (rectangles, circles)
- Blur/pixelation effects
- Cloud service integration
- Video/GIF recording