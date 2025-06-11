#!/bin/bash

# Flint Screenshot Tool Installation Script
# This script helps install Flint and its dependencies on Linux systems

set -e

echo "🔥 Flint Screenshot Tool Installation Script"
echo "============================================="

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to detect Linux distribution
detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        echo $ID
    elif [ -f /etc/redhat-release ]; then
        echo "centos"
    elif [ -f /etc/debian_version ]; then
        echo "debian"
    else
        echo "unknown"
    fi
}

# Function to install system dependencies
install_dependencies() {
    local distro=$(detect_distro)
    print_status "Detected distribution: $distro"

    case $distro in
        ubuntu|debian|pop|linuxmint)
            print_status "Installing dependencies for Debian/Ubuntu..."
            sudo apt update
            sudo apt install -y \
                build-essential \
                pkg-config \
                libgtk-4-dev \
                libcairo2-dev \
                libglib2.0-dev \
                curl \
                git
            ;;
        fedora)
            print_status "Installing dependencies for Fedora..."
            sudo dnf install -y \
                gcc \
                pkg-config \
                gtk4-devel \
                cairo-devel \
                glib2-devel \
                curl \
                git
            ;;
        arch|manjaro)
            print_status "Installing dependencies for Arch Linux..."
            sudo pacman -S --needed \
                base-devel \
                pkg-config \
                gtk4 \
                cairo \
                glib2 \
                curl \
                git
            ;;
        opensuse*)
            print_status "Installing dependencies for openSUSE..."
            sudo zypper install -y \
                gcc \
                pkg-config \
                gtk4-devel \
                cairo-devel \
                glib2-devel \
                curl \
                git
            ;;
        *)
            print_warning "Unknown distribution. Please install the following packages manually:"
            echo "  - build-essential/gcc/base-devel"
            echo "  - pkg-config"
            echo "  - GTK4 development packages"
            echo "  - Cairo development packages"
            echo "  - GLib development packages"
            echo "  - curl and git"
            read -p "Have you installed these dependencies? (y/N): " confirm
            if [[ ! $confirm =~ ^[Yy]$ ]]; then
                print_error "Please install dependencies first, then run this script again."
                exit 1
            fi
            ;;
    esac
}

# Function to install Rust
install_rust() {
    if command -v cargo &> /dev/null; then
        print_status "Rust is already installed ($(rustc --version))"
        return
    fi

    print_status "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    print_status "Rust installed successfully"
}

# Function to verify GTK4 installation
verify_gtk4() {
    print_status "Verifying GTK4 installation..."
    
    if ! pkg-config --exists gtk4; then
        print_error "GTK4 not found. Please install GTK4 development packages."
        exit 1
    fi
    
    local gtk_version=$(pkg-config --modversion gtk4)
    print_status "GTK4 version: $gtk_version"
}

# Function to build Flint
build_flint() {
    print_status "Building Flint..."
    
    # Ensure we're in the correct directory
    if [ ! -f "Cargo.toml" ]; then
        print_error "Cargo.toml not found. Please run this script from the Flint project directory."
        exit 1
    fi
    
    # Build in release mode
    cargo build --release
    
    if [ $? -eq 0 ]; then
        print_status "Flint built successfully!"
    else
        print_error "Build failed. Please check the error messages above."
        exit 1
    fi
}

# Function to install Flint
install_flint() {
    print_status "Installing Flint..."
    
    # Install the binary
    cargo install --path . --root ~/.local
    
    # Create desktop entry
    local desktop_dir="$HOME/.local/share/applications"
    mkdir -p "$desktop_dir"
    
    cat > "$desktop_dir/flint.desktop" << EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Flint
GenericName=Screenshot Tool
Comment=Fast screenshot and annotation utility for Linux
Exec=$HOME/.local/bin/flint
Icon=camera-photo
Terminal=false
Categories=Graphics;Photography;Utility;
Keywords=screenshot;capture;annotate;edit;image;
StartupNotify=true
StartupWMClass=flint
MimeType=image/png;image/jpeg;image/bmp;image/tiff;
Actions=capture;

[Desktop Action capture]
Name=Take Screenshot
Exec=$HOME/.local/bin/flint
Icon=camera-photo
EOF
    
    # Update desktop database
    if command -v update-desktop-database &> /dev/null; then
        update-desktop-database "$desktop_dir"
    fi
    
    print_status "Flint installed to ~/.local/bin/flint"
    print_status "Desktop entry created"
}

# Function to run post-install checks
post_install_checks() {
    print_status "Running post-install checks..."
    
    # Check if ~/.local/bin is in PATH
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        print_warning "~/.local/bin is not in your PATH"
        print_warning "Add the following line to your ~/.bashrc or ~/.zshrc:"
        echo "export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo ""
        print_warning "Then run: source ~/.bashrc (or restart your terminal)"
    fi
    
    # Check portal availability for Wayland
    if [ "$XDG_SESSION_TYPE" = "wayland" ]; then
        if ! systemctl --user is-active --quiet xdg-desktop-portal; then
            print_warning "xdg-desktop-portal is not running. Screenshot capture may not work on Wayland."
            print_warning "Try running: systemctl --user start xdg-desktop-portal"
        else
            print_status "xdg-desktop-portal is running (good for Wayland support)"
        fi
    fi
    
    print_status "Installation completed successfully! 🎉"
    echo ""
    echo "You can now:"
    echo "  1. Run 'flint' from the terminal"
    echo "  2. Find 'Flint' in your application menu"
    echo "  3. Set up keyboard shortcuts in your desktop environment"
    echo ""
    echo "For help and documentation, see: README.md"
}

# Main installation flow
main() {
    echo "This script will:"
    echo "  1. Install system dependencies (requires sudo)"
    echo "  2. Install Rust (if not already installed)"
    echo "  3. Build and install Flint"
    echo "  4. Create desktop entries"
    echo ""
    
    read -p "Continue? (y/N): " confirm
    if [[ ! $confirm =~ ^[Yy]$ ]]; then
        echo "Installation cancelled."
        exit 0
    fi
    
    echo ""
    
    install_dependencies
    install_rust
    verify_gtk4
    build_flint
    install_flint
    post_install_checks
}

# Handle command line arguments
case "${1:-}" in
    --deps-only)
        install_dependencies
        verify_gtk4
        ;;
    --build-only)
        verify_gtk4
        build_flint
        ;;
    --help|-h)
        echo "Flint Installation Script"
        echo ""
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --deps-only   Only install system dependencies"
        echo "  --build-only  Only build Flint (assumes deps are installed)"
        echo "  --help, -h    Show this help message"
        echo ""
        echo "Run without arguments for full installation."
        ;;
    "")
        main
        ;;
    *)
        print_error "Unknown option: $1"
        echo "Use --help for usage information."
        exit 1
        ;;
esac