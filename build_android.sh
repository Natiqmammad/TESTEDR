#!/bin/bash

# Build Script for NightScript Android Runtime
# Compiles Rust library for all Android architectures

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║    ApexForge NightScript (AFNS) Android Build Script      ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BUILD_TYPE="${1:-release}"
TARGETS=(
    "aarch64-linux-android"
    "armv7-linux-androideabi"
    "i686-linux-android"
    "x86_64-linux-android"
)

# Check if NDK is configured
check_ndk() {
    echo -e "${BLUE}[1/5]${NC} Checking Android NDK..."

    if [ -z "$ANDROID_NDK_HOME" ]; then
        echo -e "${RED}ERROR:${NC} ANDROID_NDK_HOME is not set!"
        echo "Please set ANDROID_NDK_HOME environment variable to your NDK installation path"
        echo "Example: export ANDROID_NDK_HOME=/Users/username/Library/Android/sdk/ndk/25.2.9519653"
        exit 1
    fi

    if [ ! -d "$ANDROID_NDK_HOME" ]; then
        echo -e "${RED}ERROR:${NC} NDK directory does not exist: $ANDROID_NDK_HOME"
        exit 1
    fi

    echo -e "${GREEN}✓${NC} NDK found at: $ANDROID_NDK_HOME"
}

# Check if Rust is installed
check_rust() {
    echo -e "${BLUE}[2/5]${NC} Checking Rust installation..."

    if ! command -v rustc &> /dev/null; then
        echo -e "${RED}ERROR:${NC} Rust is not installed!"
        echo "Install Rust from: https://rustup.rs/"
        exit 1
    fi

    echo -e "${GREEN}✓${NC} Rust version: $(rustc --version)"
}

# Install Android targets
install_targets() {
    echo -e "${BLUE}[3/5]${NC} Installing Android targets..."

    for target in "${TARGETS[@]}"; do
        echo -e "  ${YELLOW}→${NC} Installing $target..."
        rustup target add "$target" 2>&1 | grep -v "info:" || true
    done

    echo -e "${GREEN}✓${NC} All targets installed"
}

# Setup environment variables for cross-compilation
setup_env() {
    echo -e "${BLUE}[4/5]${NC} Setting up build environment..."

    # Determine host OS
    case "$(uname -s)" in
        Linux*)     HOST_OS=linux;;
        Darwin*)    HOST_OS=darwin;;
        *)          echo -e "${RED}ERROR:${NC} Unsupported OS"; exit 1;;
    esac

    # Determine host architecture
    case "$(uname -m)" in
        x86_64)     HOST_ARCH=x86_64;;
        arm64|aarch64) HOST_ARCH=aarch64;;
        *)          echo -e "${RED}ERROR:${NC} Unsupported architecture"; exit 1;;
    esac

    NDK_TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$HOST_OS-$HOST_ARCH"

    if [ ! -d "$NDK_TOOLCHAIN" ]; then
        echo -e "${RED}ERROR:${NC} NDK toolchain not found at: $NDK_TOOLCHAIN"
        exit 1
    fi

    # Export toolchain environment variables
    export AR_aarch64_linux_android="$NDK_TOOLCHAIN/bin/llvm-ar"
    export CC_aarch64_linux_android="$NDK_TOOLCHAIN/bin/aarch64-linux-android24-clang"
    export CXX_aarch64_linux_android="$NDK_TOOLCHAIN/bin/aarch64-linux-android24-clang++"
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_TOOLCHAIN/bin/aarch64-linux-android24-clang"

    export AR_armv7_linux_androideabi="$NDK_TOOLCHAIN/bin/llvm-ar"
    export CC_armv7_linux_androideabi="$NDK_TOOLCHAIN/bin/armv7a-linux-androideabi24-clang"
    export CXX_armv7_linux_androideabi="$NDK_TOOLCHAIN/bin/armv7a-linux-androideabi24-clang++"
    export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$NDK_TOOLCHAIN/bin/armv7a-linux-androideabi24-clang"

    export AR_i686_linux_android="$NDK_TOOLCHAIN/bin/llvm-ar"
    export CC_i686_linux_android="$NDK_TOOLCHAIN/bin/i686-linux-android24-clang"
    export CXX_i686_linux_android="$NDK_TOOLCHAIN/bin/i686-linux-android24-clang++"
    export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="$NDK_TOOLCHAIN/bin/i686-linux-android24-clang"

    export AR_x86_64_linux_android="$NDK_TOOLCHAIN/bin/llvm-ar"
    export CC_x86_64_linux_android="$NDK_TOOLCHAIN/bin/x86_64-linux-android24-clang"
    export CXX_x86_64_linux_android="$NDK_TOOLCHAIN/bin/x86_64-linux-android24-clang++"
    export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$NDK_TOOLCHAIN/bin/x86_64-linux-android24-clang"

    echo -e "${GREEN}✓${NC} Environment configured for $HOST_OS-$HOST_ARCH"
}

# Build for all targets
build_targets() {
    echo -e "${BLUE}[5/5]${NC} Building Rust library for Android..."
    echo ""

    BUILD_FLAG=""
    if [ "$BUILD_TYPE" == "release" ]; then
        BUILD_FLAG="--release"
        echo -e "Build mode: ${GREEN}RELEASE${NC}"
    else
        echo -e "Build mode: ${YELLOW}DEBUG${NC}"
    fi
    echo ""

    for target in "${TARGETS[@]}"; do
        echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${BLUE}Building for:${NC} $target"
        echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

        cargo build --lib $BUILD_FLAG --target "$target"

        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✓${NC} Successfully built for $target"
        else
            echo -e "${RED}✗${NC} Failed to build for $target"
            exit 1
        fi
        echo ""
    done
}

# Copy libraries to jniLibs
copy_to_jniLibs() {
    echo ""
    echo -e "${BLUE}[BONUS]${NC} Copying libraries to Android project..."

    JNILIBS_DIR="android/AFNSRuntime/app/src/main/jniLibs"

    if [ ! -d "$JNILIBS_DIR" ]; then
        echo -e "${YELLOW}⚠${NC}  jniLibs directory not found, creating..."
        mkdir -p "$JNILIBS_DIR"
    fi

    BUILD_DIR="debug"
    if [ "$BUILD_TYPE" == "release" ]; then
        BUILD_DIR="release"
    fi

    # Map Rust targets to Android ABIs
    declare -A ABI_MAP
    ABI_MAP["aarch64-linux-android"]="arm64-v8a"
    ABI_MAP["armv7-linux-androideabi"]="armeabi-v7a"
    ABI_MAP["i686-linux-android"]="x86"
    ABI_MAP["x86_64-linux-android"]="x86_64"

    for target in "${TARGETS[@]}"; do
        abi="${ABI_MAP[$target]}"
        src="target/$target/$BUILD_DIR/libnightscript_android.so"
        dst="$JNILIBS_DIR/$abi/"

        if [ -f "$src" ]; then
            mkdir -p "$dst"
            cp "$src" "$dst"
            echo -e "${GREEN}✓${NC} Copied $abi library"

            # Print library size
            size=$(du -h "$src" | cut -f1)
            echo -e "  Size: $size"
        else
            echo -e "${RED}✗${NC} Library not found: $src"
        fi
    done
}

# Print summary
print_summary() {
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                                                            ║${NC}"
    echo -e "${GREEN}║              ✓ BUILD COMPLETED SUCCESSFULLY                ║${NC}"
    echo -e "${GREEN}║                                                            ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Build artifacts:"
    echo ""

    BUILD_DIR="debug"
    if [ "$BUILD_TYPE" == "release" ]; then
        BUILD_DIR="release"
    fi

    for target in "${TARGETS[@]}"; do
        lib_path="target/$target/$BUILD_DIR/libnightscript_android.so"
        if [ -f "$lib_path" ]; then
            size=$(du -h "$lib_path" | cut -f1)
            echo -e "  ${GREEN}✓${NC} $target: $size"
        fi
    done

    echo ""
    echo "Next steps:"
    echo "  1. Open Android Studio"
    echo "  2. Open project: android/AFNSRuntime"
    echo "  3. Build APK: ./gradlew assembleDebug"
    echo "  4. Install: ./gradlew installDebug"
    echo ""
}

# Main execution
main() {
    check_ndk
    check_rust
    install_targets
    setup_env
    build_targets
    copy_to_jniLibs
    print_summary
}

# Run main function
main
