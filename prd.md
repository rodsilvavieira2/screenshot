# **Product Requirements Document: Flint**

|  |  |
| :---- | :---- |
| **Document Version:** | 1.0 |
| **Status:** | Proposed |
| **Author:** | Gemini |
| **Date:** | June 10, 2025 |

### **1\. Introduction & Executive Summary**

**Flint** is a high-performance, native screenshot and annotation utility designed for the modern Linux desktop. In a landscape where existing tools can be slow, resource-heavy, or lack proper Wayland support, Flint aims to provide a seamless and intuitive experience. By leveraging Rust and the GTK4 toolkit, Flint will deliver a lightweight, fast, and stable application that integrates perfectly with the GNOME desktop environment.
The core mission is to enable users to capture, annotate, and share visual information with minimal friction, moving from screen capture to a fully annotated image in seconds.

### **2\. Target Audience**

* **Technical Support & QA Engineers:** Professionals who need to document bugs, highlight errors, and create visual guides for end-users.
* **Software Developers & UI/UX Designers:** Individuals who collaborate on visual designs and need to provide clear, actionable feedback on user interfaces.
* **General Desktop Users & Students:** Anyone who needs a simple, reliable tool for capturing and marking up information for presentations, notes, or sharing on social media.

### **3\. The Problem**

Users on Linux, particularly those on modern Wayland-based systems like GNOME, often face a difficult choice for screenshot tools:

* **Legacy X11 Tools:** May not function correctly or require workarounds on Wayland.
* **Feature-Rich but Heavy Tools:** Applications built with non-native toolkits (like Electron) often suffer from slow startup times and high memory consumption.
* **Built-in Utilities:** Often lack essential post-capture editing and annotation features.

There is a clear need for a **natively built, performant, and Wayland-first** utility that provides the essential annotation tools without unnecessary bloat.

### **4\. Goals & Success Metrics**

| Goal | Success Metric |
| :---- | :---- |
| **Deliver an ultra-fast workflow** | The time from initiating capture to the editor window appearing with the image is less than 2 seconds. |
| **Ensure seamless modern desktop integration** | The application functions flawlessly on Wayland by using the xdg-desktop-portal. Provides a graceful fallback for X11 environments. |
| **Provide essential annotation tools** | The tool includes Pencil, Line/Arrow, and Highlighter functionality. |
| **Maintain high performance and low overhead** | The application's memory footprint remains under 50MB RAM during active use. |

### **5\. User Stories**

* **As a QA engineer, I want to** select a specific region of my screen, **so that I can** highlight a UI bug with a red arrow and save the image to attach to a bug report.
* **As a user, I want to** take a screenshot and immediately have the option to draw on it, **so that I can** quickly circle important information before sharing it with a colleague.
* **As a developer, I want to** capture a specific application window without other desktop elements, **so that I can** create clean documentation screenshots for my software.
* **As a GNOME user on Wayland, I want to** use a screenshot tool that invokes the native OS selection UI, **so that** the experience is consistent, secure, and reliable.
* **As a developer, I want to** copy my annotated screenshot directly to the clipboard, **so that I can** paste it instantly into a chat application or document without saving a file first.

### **6\. Features & Requirements**

#### **6.1. Core Functionality: Screenshot Capture**

* **Trigger Mechanism:** The application will present a main interface with three capture options: Screen, Selection, and Window.
* **Capture Modes:**
  * **Screen:** Captures the entire screen
  * **Selection:** Allows user to select a rectangular region
  * **Window:** Presents a list of available windows for the user to select and capture
* **Platform Support:** 
  * **X11:** Full functionality including window enumeration and capture
  * **Wayland:** Screen and Selection modes work via xdg-desktop-portal. Window selection shows an informative error due to Wayland security restrictions
  * **XWayland Applications:** Can be captured even when running on Wayland

#### **6.2. The Annotation Editor**

* **Window:** A GTK4 window will immediately appear containing the captured image. The window will resize to fit the screenshot.
* **UI:** A simple, unobtrusive toolbar will be present, providing access to all editing tools.
* **Drawing Surface:** The screenshot will be displayed on a Cairo-powered drawing area, allowing for low-level drawing operations.

#### **6.3. Annotation Tools**

| Tool | Description | Configurable Properties |
| :---- | :---- | :---- |
| **Pencil** | Allows for freehand drawing on the image. | Color, Brush Thickness (e.g., 1px, 3px, 5px). |
| **Line / Arrow** | Draws a straight line between two points. Can optionally render an arrowhead at the end point. | Color, Line Thickness, Arrowhead toggle. |
| **Highlighter** | Draws a thick, semi-transparent line to mark areas without obscuring them. | Color (presets: Yellow, Green, Pink), Brush Thickness. |

#### **6.4. Output & Export**

* **Save to File:** A "Save" button will open a native file-save dialog, allowing the user to save the annotated image as a PNG file.
* **Copy to Clipboard:** A "Copy" button will place the final annotated image onto the system clipboard for immediate pasting into other applications.

#### **6.5. Non-Functional Requirements**

| Requirement | Specification |
| :---- | :---- |
| **Platform Support** | Linux. Primary target is GNOME on Wayland with X11 support. |
| **Architecture** | x86\_64 |
| **Technology Stack** | Rust, GTK4, ashpd (for portal), Cairo, x11rb (for X11 window management), wayland-client |
| **Performance** | The application must feel responsive and lightweight at all times. No noticeable lag during drawing operations. |
| **Installation** | The project should be buildable from source via cargo. Future goal is to distribute via Flatpak. |

### **7\. Implementation Status**

#### **7.1. Completed Features (v1.0)**

* ✅ **Window Selection:** Users can select and capture individual application windows
* ✅ **X11 Window Enumeration:** Full support for listing and capturing windows on X11 systems
* ✅ **Multi-platform Support:** Graceful handling of Wayland limitations with informative error messages
* ✅ **Enhanced UI:** Three-button interface (Screen, Selection, Window) for different capture modes
* ✅ **String Sanitization:** Proper handling of window titles with special characters for GTK compatibility

#### **7.2. Platform-Specific Behavior**

* **X11 Systems:** Full window selection functionality with window enumeration and capture
* **Wayland Systems:** Screen and Selection modes available; Window selection shows helpful error message directing users to alternative capture methods
* **XWayland Applications:** Can be captured through X11 backend even when running on Wayland

### **8\. Out of Scope for Future Versions**

To ensure a focused and achievable implementation, the following features are **not** currently planned:

* Video or GIF recording.
* Direct upload to cloud services (e.g., Imgur).
* Advanced editing tools: Text overlays, shape tools (rectangles, circles), blur/pixelation effects.
* Timed or delayed screenshots.
* Multi-level undo/redo (a simple "clear all annotations" might be considered).
* Native Wayland window enumeration (due to security restrictions in the Wayland protocol).
