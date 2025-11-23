// Top-level build file where you can add configuration options common to all sub-projects/modules.
plugins {
    id("com.android.application") version "8.2.1" apply false
    id("com.android.library") version "8.2.1" apply false
    id("org.jetbrains.kotlin.android") version "1.9.20" apply false
    id("org.jetbrains.kotlin.kapt") version "1.9.20" apply false
}

buildscript {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }

    dependencies {
        classpath("com.android.tools.build:gradle:8.2.1")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.20")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
        maven { url = uri("https://jitpack.io") }
    }
}

tasks.register("clean", Delete::class) {
    delete(rootProject.buildDir)
}

// Custom tasks for NightScript Android integration

tasks.register("setupAndroidNDK") {
    group = "setup"
    description = "Verify Android NDK installation and setup"

    doLast {
        val ndkHome = System.getenv("ANDROID_NDK_HOME")
        if (ndkHome == null) {
            println("WARNING: ANDROID_NDK_HOME is not set!")
            println("Please set ANDROID_NDK_HOME environment variable to your NDK installation path")
            println("Example: export ANDROID_NDK_HOME=/Users/username/Library/Android/sdk/ndk/25.2.9519653")
        } else {
            println("ANDROID_NDK_HOME is set to: $ndkHome")
            val ndkDir = file(ndkHome)
            if (ndkDir.exists()) {
                println("✓ NDK directory exists")
            } else {
                println("✗ NDK directory does not exist!")
            }
        }
    }
}

tasks.register("setupRustTargets") {
    group = "setup"
    description = "Install Rust Android targets"

    doLast {
        exec {
            commandLine("rustup", "target", "add", "aarch64-linux-android")
        }
        exec {
            commandLine("rustup", "target", "add", "armv7-linux-androideabi")
        }
        exec {
            commandLine("rustup", "target", "add", "i686-linux-android")
        }
        exec {
            commandLine("rustup", "target", "add", "x86_64-linux-android")
        }
        println("✓ All Rust Android targets installed")
    }
}

tasks.register("verifySetup") {
    group = "setup"
    description = "Verify complete development setup"

    dependsOn("setupAndroidNDK")

    doLast {
        println("\n=== NightScript Android Development Setup Verification ===\n")

        // Check Rust
        try {
            exec {
                commandLine("rustc", "--version")
                standardOutput = System.out
            }
            println("✓ Rust is installed")
        } catch (e: Exception) {
            println("✗ Rust is not installed or not in PATH")
        }

        // Check Cargo
        try {
            exec {
                commandLine("cargo", "--version")
                standardOutput = System.out
            }
            println("✓ Cargo is installed")
        } catch (e: Exception) {
            println("✗ Cargo is not installed or not in PATH")
        }

        // Check Android SDK
        val androidHome = System.getenv("ANDROID_HOME") ?: System.getenv("ANDROID_SDK_ROOT")
        if (androidHome != null) {
            println("✓ ANDROID_HOME is set to: $androidHome")
        } else {
            println("✗ ANDROID_HOME is not set")
        }

        // Check Android NDK
        val ndkHome = System.getenv("ANDROID_NDK_HOME")
        if (ndkHome != null) {
            println("✓ ANDROID_NDK_HOME is set to: $ndkHome")
        } else {
            println("✗ ANDROID_NDK_HOME is not set")
        }

        println("\n=== Setup Verification Complete ===\n")
    }
}

tasks.register("printBuildInfo") {
    group = "help"
    description = "Print build information"

    doLast {
        println("""

            ╔════════════════════════════════════════════════════════════╗
            ║                                                            ║
            ║    ApexForge NightScript (AFNS) Android Runtime           ║
            ║                                                            ║
            ║    Version: 1.0.0-alpha                                    ║
            ║    Architecture: Kotlin/Java ↔ JNI ↔ Rust ↔ NightScript  ║
            ║                                                            ║
            ╚════════════════════════════════════════════════════════════╝

            Build Configuration:
              - Kotlin: 1.9.20
              - Android Gradle Plugin: 8.2.1
              - Min SDK: 24 (Android 7.0)
              - Target SDK: 34 (Android 14)
              - Compile SDK: 34

            Supported ABIs:
              - arm64-v8a (64-bit ARM)
              - armeabi-v7a (32-bit ARM)
              - x86 (32-bit Intel)
              - x86_64 (64-bit Intel)

            Native Libraries:
              - libnightscript_android.so (Rust JNI bridge)

            Quick Start:
              1. Run: ./gradlew verifySetup
              2. Build Rust: ./gradlew buildRustLib
              3. Build APK: ./gradlew assembleDebug
              4. Install: ./gradlew installDebug

            Documentation:
              - README: ../../../README.md
              - ROADMAP: ../../../ROADMAP.md
              - API Docs: docs/API.md

        """.trimIndent())
    }
}

// Print build info on every build
gradle.projectsEvaluated {
    println("\n🚀 Building ApexForge NightScript Android Runtime...\n")
}
