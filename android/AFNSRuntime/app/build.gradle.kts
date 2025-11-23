plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("kotlin-kapt")
}

android {
    namespace = "com.nightscript.afns"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.nightscript.afns"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0-alpha"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        // NDK Configuration
        ndk {
            abiFilters.addAll(listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64"))
        }

        // Native library configuration
        externalNativeBuild {
            cmake {
                cppFlags += listOf("-std=c++17", "-frtti", "-fexceptions")
                arguments += listOf(
                    "-DANDROID_STL=c++_shared",
                    "-DANDROID_TOOLCHAIN=clang",
                    "-DANDROID_PLATFORM=android-24"
                )
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )

            // Native library stripping
            ndk {
                debugSymbolLevel = "FULL"
            }
        }

        debug {
            isDebuggable = true
            isJniDebuggable = true

            // Enable NDK debugging
            ndk {
                debugSymbolLevel = "FULL"
            }
        }
    }

    // NDK Build Configuration
    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }

    // Source sets for native libraries
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
        freeCompilerArgs += listOf(
            "-Xopt-in=kotlin.RequiresOptIn",
            "-Xopt-in=kotlin.ExperimentalStdlibApi"
        )
    }

    buildFeatures {
        viewBinding = true
        buildConfig = true
    }

    // Packaging options for native libraries
    packagingOptions {
        jniLibs {
            useLegacyPackaging = false
        }
        resources {
            excludes += listOf(
                "META-INF/NOTICE",
                "META-INF/LICENSE",
                "META-INF/DEPENDENCIES",
                "META-INF/*.kotlin_module"
            )
        }
    }

    // Lint options
    lint {
        abortOnError = false
        checkReleaseBuilds = false
        disable += listOf("MissingTranslation", "ExtraTranslation")
    }

    // Test options
    testOptions {
        unitTests {
            isIncludeAndroidResources = true
            isReturnDefaultValues = true
        }
    }
}

dependencies {
    // Kotlin
    implementation("org.jetbrains.kotlin:kotlin-stdlib:1.9.20")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")

    // AndroidX Core
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("androidx.activity:activity-ktx:1.8.2")
    implementation("androidx.fragment:fragment-ktx:1.6.2")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.7.0")
    implementation("androidx.lifecycle:lifecycle-livedata-ktx:2.7.0")

    // Material Design
    implementation("com.google.android.material:material:1.11.0")
    implementation("androidx.constraintlayout:constraintlayout:2.1.4")

    // WorkManager (for background tasks)
    implementation("androidx.work:work-runtime-ktx:2.9.0")

    // Permissions
    implementation("androidx.activity:activity-ktx:1.8.2")
    implementation("androidx.fragment:fragment-ktx:1.6.2")

    // JSON
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.2")
    implementation("com.google.code.gson:gson:2.10.1")

    // Networking (optional, for NightScript HTTP client)
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.squareup.retrofit2:retrofit:2.9.0")
    implementation("com.squareup.retrofit2:converter-gson:2.9.0")

    // Image Loading (optional, for UI)
    implementation("io.coil-kt:coil:2.5.0")

    // Storage
    implementation("androidx.datastore:datastore-preferences:1.0.0")

    // Camera (optional)
    implementation("androidx.camera:camera-core:1.3.1")
    implementation("androidx.camera:camera-camera2:1.3.1")
    implementation("androidx.camera:camera-lifecycle:1.3.1")
    implementation("androidx.camera:camera-view:1.3.1")

    // Location (optional)
    implementation("com.google.android.gms:play-services-location:21.1.0")

    // Sensors (optional)
    implementation("androidx.preference:preference-ktx:1.2.1")

    // Testing
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlin:kotlin-test:1.9.20")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.7.3")
    testImplementation("androidx.test:core:1.5.0")
    testImplementation("androidx.test:runner:1.5.2")
    testImplementation("androidx.test:rules:1.5.0")
    testImplementation("androidx.arch.core:core-testing:2.2.0")

    // Android Testing
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
    androidTestImplementation("androidx.test:runner:1.5.2")
    androidTestImplementation("androidx.test:rules:1.5.0")
    androidTestImplementation("androidx.test.uiautomator:uiautomator:2.3.0")

    // Memory Leak Detection (debug only)
    debugImplementation("com.squareup.leakcanary:leakcanary-android:2.13")

    // Logging (debug only)
    debugImplementation("com.jakewharton.timber:timber:5.0.1")
}

// Task to copy Rust library to jniLibs
tasks.register<Copy>("copyRustLibs") {
    description = "Copy Rust compiled libraries to jniLibs directory"
    group = "build"

    from("${project.rootDir}/../../target/aarch64-linux-android/release") {
        include("libnightscript_android.so")
        into("arm64-v8a")
    }
    from("${project.rootDir}/../../target/armv7-linux-androideabi/release") {
        include("libnightscript_android.so")
        into("armeabi-v7a")
    }
    from("${project.rootDir}/../../target/i686-linux-android/release") {
        include("libnightscript_android.so")
        into("x86")
    }
    from("${project.rootDir}/../../target/x86_64-linux-android/release") {
        include("libnightscript_android.so")
        into("x86_64")
    }

    into("${projectDir}/src/main/jniLibs")
}

// Task to build Rust library
tasks.register<Exec>("buildRustLib") {
    description = "Build NightScript Rust library for Android"
    group = "build"

    workingDir = file("${project.rootDir}/../..")

    commandLine(
        "cargo", "build",
        "--release",
        "--lib",
        "--target", "aarch64-linux-android",
        "--target", "armv7-linux-androideabi",
        "--target", "i686-linux-android",
        "--target", "x86_64-linux-android"
    )

    // Set environment variables
    environment(
        "ANDROID_NDK_HOME" to System.getenv("ANDROID_NDK_HOME"),
        "AR_aarch64_linux_android" to "${System.getenv("ANDROID_NDK_HOME")}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar",
        "CC_aarch64_linux_android" to "${System.getenv("ANDROID_NDK_HOME")}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang",
        "CXX_aarch64_linux_android" to "${System.getenv("ANDROID_NDK_HOME")}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang++",
        "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER" to "${System.getenv("ANDROID_NDK_HOME")}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang"
    )

    doLast {
        println("Rust library built successfully")
    }
}

// Make assembleDebug and assembleRelease depend on Rust build
tasks.named("preBuild") {
    dependsOn("buildRustLib", "copyRustLibs")
}

// Clean task for Rust
tasks.register<Exec>("cleanRust") {
    description = "Clean Rust build artifacts"
    group = "build"

    workingDir = file("${project.rootDir}/../..")
    commandLine("cargo", "clean")
}

tasks.named("clean") {
    dependsOn("cleanRust")
}
