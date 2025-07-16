# Package metadata
pkgname=oca                    # Package name, used by pacman
pkgver=0.1.0                   # Initial version; bump when releasing new versions
pkgrel=1                       # Package release; increment when PKGBUILD changes
pkgdesc="Ollama CLI Assistant"  # Short description of the package
arch=('x86_64')                # Supported architectures
url="https://github.com/Ezpcy/ollama-cli-assistant"  # Project homepage
license=('')                # License type

# Build and runtime dependencies
makedepends=('cargo' 'rust')  # Needed to compile the Rust project
depends=('')           # Runtime dependencies (adjust as necessary)

# Source: Git clone of your repository at the matching version tag
source=("git+https://github.com/ezpcy/ollama-cli-assistant.git#tag=v${pkgver}")
sha256sums=('SKIP')            # SKIP for development/local packages

# Build the project using Cargo
build() {
  cd "$srcdir/ollama-cli-assistant"
  cargo build --release --locked    # --locked ensures Cargo.lock is honored
}

# Install the binary and make it invocable as 'oca'
package() {
  cd "$srcdir/ollama-cli-assistant"
  install -Dm755 \
    target/release/ollama-cli-assistant \
    "$pkgdir/usr/bin/oca"         # Installs as /usr/bin/oca
}

# vim: set ts=2 sw=2 et:

