# 🤖 Android Build & Development Guide

**ApexForge NightScript (AFNS) - Android JNI Integration**

This guide covers everything you need to build, deploy, and develop NightScript applications for Android.

---

## 📋 Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start](#quick-start)
3. [Environment Setup](#environment-setup)
4. [Building the Native Library](#building-the-native-library)
5. [Android Studio Setup](#android-studio-setup)
6. [Building the APK](#building-the-apk)
7. [Running on Device/Emulator](#running-on-deviceemulator)
8. [Development Workflow](#development-workflow)
9. [Debugging](#debugging)
10. [Troubleshooting](#troubleshooting)
11. [Architecture Overview](#architecture-overview)

---

## Prerequisites

### Required Software

1. **Rust** (1.70+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Android Studio** (Latest stable version)
   - Download from: https://developer.android.com/studio

3. **Android NDK** (25.2.9519653 or later)
   - Install via Android Studio SDK Manager

4. **Java Development Kit** (JDK 17)
   ```bash
   # macOS
   brew install openjdk@17
   
   # Ubuntu/Debian
   sudo apt install openjdk-17-jdk
   ```

### System Requirements

- **Operating System:** Linux, macOS, or Windows (WSL2 recommended)
- **RAM:** 8GB minimum, 16GB recommended
- **Disk Space:** 10GB free space
- **CPU:** 64-bit processor

---

## Quick Start

### One-Command Build

```bash
# Build Rust library for all Android architectures
./build_android.sh release

# Build APK
cd android/AFNSRuntime
./gradlew assembleDebug

# Install on connected device
./gradlew installDebug
```

---

## Environment Setup

### 1. Set Environment Variables

Add to your `~/.bashrc`, `~/.zshrc`, or `~/.profile`:

```bash
# Android SDK
export ANDROID_HOME="$HOME/Library/Android/sdk"  # macOS
# export ANDROID_HOME="$HOME/Android/Sdk"        # Linux

# Android NDK
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/25.2.9519653"

# Add to PATH
export PATH="$ANDROID_HOME/platform-tools:$PATH"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
```

Apply changes:
```bash
source ~/.bashrc  # or ~/.zshrc
```

### 2. Verify Installation

```bash
# Check Android SDK
which adb
# Output: /path/to/sdk/platform-tools/adb

# Check NDK
ls "$ANDROID_NDK_HOME"
# Should show: build/, toolchains/, etc.

# Check Rust
rustc --version
cargo --version
```

### 3. Install Android Rust Targets

```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
rustup target add x86_64-linux-android
```

Verify:
```bash
rustup target list | grep android
```

---

## Building the Native Library

### Automated Build (Recommended)

```bash
# Release build (optimized, smaller size)
./build_android.sh release

# Debug build (faster compilation, symbols included)
./build_android.sh debug
```

### Manual Build

```bash
# Set up NDK toolchain paths
export NDK="$ANDROID_NDK_HOME"
export TOOLCHAIN="$NDK/toolchains/llvm/prebuilt/linux-x86_64"  # Adjust for your OS

# Build for each architecture
cargo build --lib --release --target aarch64-linux-android
cargo build --lib --release --target armv7-linux-androideabi
cargo build --lib --release --target i686-linux-android
cargo build --lib --release --target x86_64-linux-android
```

### Copy Libraries to Android Project

```bash
# Automated (already done by build_android.sh)
# Or manually:
mkdir -p android/AFNSRuntime/app/src/main/jniLibs/arm64-v8a
cp target/aarch64-linux-android/release/libnightscript_android.so \
   android/AFNSRuntime/app/src/main/jniLibs/arm64-v8a/

mkdir -p android/AFNSRuntime/app/src/main/jniLibs/armeabi-v7a
cp target/armv7-linux-androideabi/release/libnightscript_android.so \
   android/AFNSRuntime/app/src/main/jniLibs/armeabi-v7a/

mkdir -p android/AFNSRuntime/app/src/main/jniLibs/x86
cp target/i686-linux-android/release/libnightscript_android.so \
   android/AFNSRuntime/app/src/main/jniLibs/x86/

mkdir -p android/AFNSRuntime/app/src/main/jniLibs/x86_64
cp target/x86_64-linux-android/release/libnightscript_android.so \
   android/AFNSRuntime/app/src/main/jniLibs/x86_64/
```

---

## Android Studio Setup

### 1. Open Project

1. Launch Android Studio
2. Click "Open an Existing Project"
3. Navigate to: `NightScript/android/AFNSRuntime`
4. Click "OK"

### 2. Sync Gradle

Wait for Gradle sync to complete (may take a few minutes on first run).

If you see errors:
- Click "File" → "Sync Project with Gradle Files"
- Or click the sync icon in the toolbar

### 3. Configure SDK

1. Go to "Tools" → "SDK Manager"
2. Ensure these are installed:
   - Android SDK Platform 34
   - Android SDK Build-Tools 34.0.0
   - Android Emulator
   - Android SDK Platform-Tools
   - NDK (Side by side) 25.2.9519653

### 4. Project Structure

```
android/AFNSRuntime/
├── app/
│   ├── src/
│   │   ├── main/
│   │   │   ├── java/com/nightscript/afns/
│   │   │   │   ├── AFNSActivity.kt
│   │   │   │   ├── AFNSApplication.kt
│   │   │   │   ├── NativeBridge.kt
│   │   │   │   ├── PermissionManager.kt
│   │   │   │   ├── IntentRouter.kt
│   │   │   │   ├── ServiceManager.kt
│   │   │   │   └── StorageManager.kt
│   │   │   ├── jniLibs/
│   │   │   │   ├── arm64-v8a/libnightscript_android.so
│   │   │   │   ├── armeabi-v7a/libnightscript_android.so
│   │   │   │   ├── x86/libnightscript_android.so
│   │   │   │   └── x86_64/libnightscript_android.so
│   │   │   └── AndroidManifest.xml
│   │   └── test/
│   └── build.gradle.kts
├── build.gradle.kts
└── settings.gradle.kts
```

---

## Building the APK

### Using Gradle (Command Line)

```bash
cd android/AFNSRuntime

# Debug build (faster, includes symbols)
./gradlew assembleDebug

# Release build (optimized, signed)
./gradlew assembleRelease

# Output:
# app/build/outputs/apk/debug/app-debug.apk
# app/build/outputs/apk/release/app-release.apk
```

### Using Android Studio

1. Select "Build" → "Build Bundle(s) / APK(s)" → "Build APK(s)"
2. Wait for build to complete
3. Click "Locate" in the notification to find the APK

### Build Variants

```bash
# List all build variants
./gradlew tasks --all | grep assemble

# Build specific variant
./gradlew assembleDebug
./gradlew assembleRelease
```

---

## Running on Device/Emulator

### Physical Device

#### 1. Enable Developer Options

1. Go to "Settings" → "About Phone"
2. Tap "Build Number" 7 times
3. Go back to "Settings" → "Developer Options"
4. Enable "USB Debugging"

#### 2. Connect Device

```bash
# Connect via USB
adb devices

# Should show:
# List of devices attached
# XXXXXXXX    device
```

If device not shown:
```bash
# Kill and restart ADB server
adb kill-server
adb start-server
adb devices
```

#### 3. Install APK

```bash
# Using Gradle
./gradlew installDebug

# Or using ADB directly
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

#### 4. Run Application

```bash
# Launch main activity
adb shell am start -n com.nightscript.afns/.AFNSActivity

# Or tap the app icon on device
```

### Emulator

#### 1. Create AVD (Android Virtual Device)

Using Android Studio:
1. Go to "Tools" → "Device Manager"
2. Click "Create Device"
3. Select hardware (e.g., Pixel 6)
4. Select system image (API 34 recommended)
5. Click "Finish"

Using Command Line:
```bash
# List available system images
sdkmanager --list | grep system-images

# Download system image
sdkmanager "system-images;android-34;google_apis;x86_64"

# Create AVD
avdmanager create avd \
  -n Pixel_6_API_34 \
  -k "system-images;android-34;google_apis;x86_64" \
  -d "pixel_6"
```

#### 2. Start Emulator

```bash
# List available AVDs
emulator -list-avds

# Start emulator
emulator -avd Pixel_6_API_34

# Or with specific options
emulator -avd Pixel_6_API_34 -no-snapshot-load -gpu host
```

#### 3. Install and Run

```bash
# Wait for emulator to boot
adb wait-for-device

# Install APK
./gradlew installDebug

# Launch app
adb shell am start -n com.nightscript.afns/.AFNSActivity
```

---

## Development Workflow

### Typical Development Cycle

```bash
# 1. Make changes to Rust code
vim src/runtime/android/mod.rs

# 2. Rebuild native library
./build_android.sh release

# 3. Sync Android Studio (if open)
# or build APK
cd android/AFNSRuntime
./gradlew assembleDebug

# 4. Install on device
./gradlew installDebug

# 5. View logs
adb logcat | grep -E "AFNS|NightScript|AndroidRuntime"
```

### Hot Reload (Kotlin Only)

Android Studio supports hot reload for Kotlin code:
1. Make changes to Kotlin files
2. Click "Apply Changes" (⌘\\ on Mac, Ctrl+\\ on Windows)

**Note:** Rust changes require full rebuild.

### Incremental Builds

```bash
# Build only changed targets
cargo build --target aarch64-linux-android --release

# Skip Rust build if no changes
cd android/AFNSRuntime
./gradlew assembleDebug --offline
```

---

## Debugging

### Logcat Filtering

```bash
# View all app logs
adb logcat -s AFNSActivity AFNSApplication NativeBridge AndroidRuntime

# Filter by priority
adb logcat *:E  # Errors only
adb logcat *:W  # Warnings and above
adb logcat *:I  # Info and above

# Filter by tag
adb logcat | grep AFNS

# Save to file
adb logcat > logs.txt
```

### Native Debugging (LLDB)

```bash
# Enable debugging in Cargo.toml
[profile.release]
debug = true

# Rebuild with symbols
./build_android.sh debug

# Attach LLDB via Android Studio
# 1. Run app in debug mode
# 2. Tools → Attach Debugger to Android Process
# 3. Select app and "Native Only"
```

### Crash Reports

```bash
# View crash logs
adb logcat | grep -E "FATAL|AndroidRuntime"

# Pull crash dumps
adb pull /data/tombstones/

# Stack trace
adb logcat -b crash
```

### Performance Profiling

```bash
# CPU profiler
# In Android Studio: View → Tool Windows → Profiler

# Memory profiler
# In Android Studio: View → Tool Windows → Profiler → Memory

# Network profiler
# In Android Studio: View → Tool Windows → Profiler → Network
```

---

## Troubleshooting

### Common Issues

#### 1. "JVM not available" Error

**Problem:** JNI bridge not initialized

**Solution:**
```bash
# Check library is loaded
adb shell run-as com.nightscript.afns ls lib/

# Should show: libnightscript_android.so

# If missing, rebuild and reinstall
./build_android.sh release
cd android/AFNSRuntime
./gradlew installDebug
```

#### 2. "Native library not found"

**Problem:** Library not copied to jniLibs

**Solution:**
```bash
# Verify libraries exist
ls android/AFNSRuntime/app/src/main/jniLibs/arm64-v8a/

# If missing:
./build_android.sh release

# Or manually copy
cp target/aarch64-linux-android/release/libnightscript_android.so \
   android/AFNSRuntime/app/src/main/jniLibs/arm64-v8a/
```

#### 3. Build Fails with "NDK not found"

**Problem:** ANDROID_NDK_HOME not set

**Solution:**
```bash
# Set NDK path
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/25.2.9519653"

# Verify
echo $ANDROID_NDK_HOME
ls "$ANDROID_NDK_HOME"
```

#### 4. ABI Mismatch

**Problem:** App crashes immediately on start

**Solution:**
```bash
# Check device ABI
adb shell getprop ro.product.cpu.abi

# Build for specific ABI
cargo build --lib --release --target aarch64-linux-android

# Copy to correct folder
cp target/aarch64-linux-android/release/libnightscript_android.so \
   android/AFNSRuntime/app/src/main/jniLibs/arm64-v8a/
```

#### 5. Permission Denied

**Problem:** Can't access storage or camera

**Solution:**
- Check AndroidManifest.xml includes required permissions
- Request runtime permissions in code
- Grant permissions manually: Settings → Apps → AFNS → Permissions

#### 6. Gradle Sync Failed

**Problem:** Dependencies can't be downloaded

**Solution:**
```bash
# Clear Gradle cache
cd android/AFNSRuntime
./gradlew clean

# Or delete cache
rm -rf ~/.gradle/caches/

# Sync again
./gradlew --refresh-dependencies
```

---

## Architecture Overview

### Components

```
┌─────────────────────────────────────────────────────────┐
│                    Android Application                   │
│  ┌───────────────────────────────────────────────────┐  │
│  │           Kotlin/Java Layer                       │  │
│  │  - AFNSActivity (UI, Lifecycle)                  │  │
│  │  - NativeBridge (JNI Gateway)                    │  │
│  │  - Managers (Permissions, Intents, etc.)         │  │
│  └───────────────┬───────────────────────────────────┘  │
│                  │ JNI Calls                             │
│  ┌───────────────▼───────────────────────────────────┐  │
│  │        JNI Bridge (Rust)                          │  │
│  │  - Type conversions (Java ↔ Rust)               │  │
│  │  - Lifecycle callbacks                            │  │
│  │  - Platform channel routing                       │  │
│  └───────────────┬───────────────────────────────────┘  │
│                  │ FFI Calls                             │
│  ┌───────────────▼───────────────────────────────────┐  │
│  │      NightScript Runtime (Rust)                   │  │
│  │  - Interpreter                                    │  │
│  │  - Standard library (forge.*)                     │  │
│  │  - Memory management                              │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Call Flow Example

**NightScript → Android Toast:**

```
1. NightScript code:
   android.Context.show_toast("Hello")

2. Runtime interprets and calls:
   builtin_android_context_show_toast()

3. Rust calls JNI:
   env.call_method(activity, "showToast", ...)

4. Kotlin receives call:
   AFNSActivity.showToast(message)

5. Android displays toast:
   Toast.makeText(this, message, LENGTH_SHORT).show()
```

### Memory Management

- **Java Objects:** Managed by JVM garbage collector
- **Rust Values:** RAII-based (no GC)
- **JNI Global Refs:** Must be manually released
- **Local Refs:** Automatically cleaned up after JNI call

---

## Performance Tips

### 1. Optimize Build Time

```bash
# Use --release only for final builds
./build_android.sh debug  # Faster compilation

# Build for single ABI during development
cargo build --target aarch64-linux-android
```

### 2. Reduce APK Size

```toml
# In Cargo.toml
[profile.release]
opt-level = "z"  # Optimize for size
lto = true       # Link-time optimization
strip = true     # Remove debug symbols
```

### 3. Minimize JNI Overhead

- Cache method IDs and class references
- Use direct ByteBuffers for large data transfers
- Batch JNI calls when possible

### 4. Profile and Monitor

```bash
# CPU usage
adb shell top | grep nightscript

# Memory usage
adb shell dumpsys meminfo com.nightscript.afns

# Battery usage
adb shell dumpsys batterystats | grep nightscript
```

---

## Additional Resources

### Documentation

- [Rust JNI Crate](https://docs.rs/jni/latest/jni/)
- [Android NDK Guide](https://developer.android.com/ndk/guides)
- [JNI Specification](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/)

### Tools

- [Android Studio](https://developer.android.com/studio)
- [scrcpy](https://github.com/Genymobile/scrcpy) - Screen mirroring
- [pidcat](https://github.com/JakeWharton/pidcat) - Colored logcat

### Community

- GitHub Issues: Report bugs and request features
- Discussions: Ask questions and share ideas

---

## Next Steps

1. ✅ **Build successful?** → Try modifying `examples/android_test.afml`
2. 🚀 **Ready to develop?** → Read `ROADMAP.md` for feature roadmap
3. 🐛 **Found a bug?** → Report on GitHub Issues
4. 💡 **Have an idea?** → Start a discussion

---

**Happy Coding! 🎉**