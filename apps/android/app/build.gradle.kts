plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.enclave.app"
    compileSdk = 36

    defaultConfig {
        // Same applicationId and data directory as the shipping Tauri app so an
        // existing vault is picked up unchanged (`applicationInfo.dataDir`).
        applicationId = "com.enclave.app"
        minSdk = 24
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0-native"

        ndk {
            // arm64 phones; x86_64 is the emulator target. ONNX Runtime ships no
            // x86_64-android binaries, so offline embeddings stay arm64-only.
            abiFilters += listOf("arm64-v8a", "x86_64")
        }

        // P2P sync runs over ws:// on the LAN and every frame is AEAD-sealed at
        // the application layer, so the platform cleartext flag must be allowed.
        manifestPlaceholders["usesCleartextTraffic"] = "true"
    }

    buildTypes {
        getByName("debug") {
            isMinifyEnabled = false
            isJniDebuggable = true
        }
        getByName("release") {
            // R8 must not strip JNA's or UniFFI's native hooks (the v1.10.1
            // launch crash); rules live in proguard-rules.pro.
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        compose = true
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }

    // The canonical block-schema fixture is owned by the core; the editor test
    // reads it directly from there so the contract has a single source.
    sourceSets {
        getByName("test") {
            resources.srcDir("../../../core/core-db/tests/fixtures")
        }
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.12.01")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-core")
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")

    // UniFFI Kotlin bindings load libcore_api.so through JNA.
    implementation("net.java.dev.jna:jna:5.14.0@aar")

    debugImplementation("androidx.compose.ui:ui-tooling")

    testImplementation("junit:junit:4.13.2")
    // The mockable android.jar ships org.json stubs that throw "Stub!"; the
    // real implementation is used so the editor model can be tested on the JVM.
    testImplementation("org.json:json:20250107")
}
