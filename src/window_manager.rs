use anyhow::{anyhow, Result};
use log::{info, warn};

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: u64,
    pub title: String,
    pub class: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_minimized: bool,
}

pub struct WindowManager {
    backend: WindowBackend,
}

enum WindowBackend {
    X11(X11WindowManager),
    Wayland(WaylandWindowManager),
}

impl WindowManager {
    pub fn new() -> Result<Self> {
        // Detect the display server
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            info!("Wayland session detected, using Wayland backend");
            match WaylandWindowManager::new() {
                Ok(backend) => Ok(Self {
                    backend: WindowBackend::Wayland(backend),
                }),
                Err(e) => {
                    warn!(
                        "Failed to initialize Wayland backend: {}, falling back to X11",
                        e
                    );
                    Ok(Self {
                        backend: WindowBackend::X11(X11WindowManager::new()?),
                    })
                }
            }
        } else {
            info!("X11 session detected, using X11 backend");
            Ok(Self {
                backend: WindowBackend::X11(X11WindowManager::new()?),
            })
        }
    }

    pub fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        match &self.backend {
            WindowBackend::X11(manager) => manager.list_windows(),
            WindowBackend::Wayland(manager) => manager.list_windows(),
        }
    }

    pub fn capture_window(&self, window_id: u64) -> Result<Vec<u8>> {
        match &self.backend {
            WindowBackend::X11(manager) => manager.capture_window(window_id),
            WindowBackend::Wayland(manager) => manager.capture_window(window_id),
        }
    }
}

// X11 Window Manager Implementation
struct X11WindowManager {
    #[cfg(feature = "x11")]
    connection: Option<x11rb::rust_connection::RustConnection>,
}

impl X11WindowManager {
    fn new() -> Result<Self> {
        #[cfg(feature = "x11")]
        {
            match x11rb::connect(None) {
                Ok((conn, _)) => Ok(Self {
                    connection: Some(conn),
                }),
                Err(e) => Err(anyhow!("Failed to connect to X11 server: {}", e)),
            }
        }
        #[cfg(not(feature = "x11"))]
        {
            Err(anyhow!("X11 support not compiled in"))
        }
    }

    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        #[cfg(feature = "x11")]
        {
            use x11rb::connection::Connection;
            use x11rb::protocol::xproto::*;

            let conn = self
                .connection
                .as_ref()
                .ok_or_else(|| anyhow!("No X11 connection"))?;
            let screen = &conn.setup().roots[0];
            let root = screen.root;

            // Query the window tree
            let tree_reply = conn.query_tree(root)?.reply()?;
            let mut windows = Vec::new();

            for &window_id in &tree_reply.children {
                if let Ok(window_info) = self.get_window_info(conn, window_id) {
                    // Filter out windows that shouldn't be captured
                    if !window_info.title.is_empty()
                        && !window_info.is_minimized
                        && window_info.width > 50
                        && window_info.height > 50
                    {
                        windows.push(window_info);
                    }
                }
            }

            Ok(windows)
        }
        #[cfg(not(feature = "x11"))]
        {
            Err(anyhow!("X11 support not compiled in"))
        }
    }

    #[cfg(feature = "x11")]
    fn get_window_info(
        &self,
        conn: &impl x11rb::connection::Connection,
        window_id: u32,
    ) -> Result<WindowInfo> {
        use x11rb::protocol::xproto::{ConnectionExt, MapState};

        // Get window geometry
        let geom_reply = conn.get_geometry(window_id)?.reply()?;

        // Get window attributes
        let attrs_reply = conn.get_window_attributes(window_id)?.reply()?;

        // Skip invisible or unmapped windows
        if attrs_reply.map_state != MapState::VIEWABLE {
            return Err(anyhow!("Window not viewable"));
        }

        // Get window title
        let title = self
            .get_window_title(conn, window_id)
            .unwrap_or_else(|_| "Unknown".to_string());

        // Get window class
        let class = self
            .get_window_class(conn, window_id)
            .unwrap_or_else(|_| "Unknown".to_string());

        Ok(WindowInfo {
            id: window_id as u64,
            title,
            class,
            x: geom_reply.x as i32,
            y: geom_reply.y as i32,
            width: geom_reply.width as u32,
            height: geom_reply.height as u32,
            is_minimized: false, // We already filtered out non-viewable windows
        })
    }

    #[cfg(feature = "x11")]
    fn get_window_title(
        &self,
        conn: &impl x11rb::connection::Connection,
        window_id: u32,
    ) -> Result<String> {
        // Try _NET_WM_NAME first (UTF-8)
        if let Ok(title) = self.get_text_property(conn, window_id, b"_NET_WM_NAME") {
            if !title.is_empty() {
                return Ok(title);
            }
        }

        // Fallback to WM_NAME
        self.get_text_property(conn, window_id, b"WM_NAME")
    }

    #[cfg(feature = "x11")]
    fn get_window_class(
        &self,
        conn: &impl x11rb::connection::Connection,
        window_id: u32,
    ) -> Result<String> {
        self.get_text_property(conn, window_id, b"WM_CLASS")
    }

    #[cfg(feature = "x11")]
    fn get_text_property(
        &self,
        conn: &impl x11rb::connection::Connection,
        window_id: u32,
        property_name: &[u8],
    ) -> Result<String> {
        use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

        let atom = conn.intern_atom(false, property_name)?.reply()?.atom;
        let reply = conn
            .get_property(false, window_id, atom, AtomEnum::ANY, 0, 1024)?
            .reply()?;

        if reply.value.is_empty() {
            return Err(anyhow!("Property not found"));
        }

        // Handle different text encodings
        let text = if reply.format == 8 {
            String::from_utf8_lossy(&reply.value)
                .trim_end_matches('\0')
                .replace('\0', "") // Remove any remaining null characters
                .trim()
                .to_string()
        } else {
            return Err(anyhow!("Unsupported text format"));
        };

        // Ensure we don't return empty strings
        if text.is_empty() {
            return Err(anyhow!("Property is empty"));
        }

        Ok(text)
    }

    fn capture_window(&self, window_id: u64) -> Result<Vec<u8>> {
        #[cfg(feature = "x11")]
        {
            use x11rb::connection::Connection;
            use x11rb::protocol::xproto::*;

            let conn = self
                .connection
                .as_ref()
                .ok_or_else(|| anyhow!("No X11 connection"))?;
            let window_id = window_id as u32;

            // Get window geometry
            let geom_reply = conn.get_geometry(window_id)?.reply()?;

            // Translate coordinates to root window
            let translate_reply = conn
                .translate_coordinates(window_id, conn.setup().roots[0].root, 0, 0)?
                .reply()?;

            let x = translate_reply.dst_x;
            let y = translate_reply.dst_y;
            let width = geom_reply.width;
            let height = geom_reply.height;

            info!("Capturing window {}x{} at ({}, {})", width, height, x, y);

            // Use the existing screenshot functionality with region capture
            let capture = crate::capture::ScreenshotCapture::new();
            capture.take_screenshot_region_blocking(x as i32, y as i32, width as i32, height as i32)
        }
        #[cfg(not(feature = "x11"))]
        {
            Err(anyhow!("X11 support not compiled in"))
        }
    }
}

// Wayland Window Manager Implementation
struct WaylandWindowManager {
    #[cfg(feature = "wayland")]
    _connection: Option<()>, // Placeholder for Wayland connection
}

impl WaylandWindowManager {
    fn new() -> Result<Self> {
        #[cfg(feature = "wayland")]
        {
            // For now, we'll return an error as Wayland window enumeration
            // is complex and requires compositor-specific protocols
            warn!("Wayland window enumeration is not fully implemented");
            Err(anyhow!(
                "Wayland window enumeration not yet supported. Window selection works only on X11."
            ))
        }
        #[cfg(not(feature = "wayland"))]
        {
            Err(anyhow!("Wayland support not compiled in"))
        }
    }

    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        #[cfg(feature = "wayland")]
        {
            // On Wayland, window enumeration is restricted for security reasons.
            // Most compositors don't provide a way to list all windows.
            // This would require compositor-specific protocols or using portals.
            Err(anyhow!("Wayland window enumeration is not supported due to security restrictions. Use X11 or select a screen region instead."))
        }
        #[cfg(not(feature = "wayland"))]
        {
            Err(anyhow!("Wayland support not compiled in"))
        }
    }

    fn capture_window(&self, _window_id: u64) -> Result<Vec<u8>> {
        #[cfg(feature = "wayland")]
        {
            Err(anyhow!(
                "Wayland window capture is not supported. Use screen or region capture instead."
            ))
        }
        #[cfg(not(feature = "wayland"))]
        {
            Err(anyhow!("Wayland support not compiled in"))
        }
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback to a dummy implementation if initialization fails
            Self {
                backend: WindowBackend::X11(X11WindowManager {
                    #[cfg(feature = "x11")]
                    connection: None,
                }),
            }
        })
    }
}
