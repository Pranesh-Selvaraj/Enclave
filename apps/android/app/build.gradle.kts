import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

// The native app's version lives in version.properties; a -P flag wins when a
// build should carry a release tag's version.
val nativeVersion = Properties().apply {
    val file = rootProject.file("version.properties")
    if (file.exists()) file.inputStream().use { load(it) }
}
val appVersionName = (project.findProperty("appVersionName") as String?)
    ?: nativeVersion.getProperty("versionName", "0.0.0-dev")
val appVersionCode = ((project.findProperty("appVersionCode") as String?)
    ?: nativeVersion.getProperty("versionCode", "1")).toInt()

fun gitSha(): String = runCatching {
    ProcessBuilder("git", "rev-parse", "--short", "HEAD")
        .directory(rootProject.projectDir)
        .redirectErrorStream(true)
        .start().inputStream.bufferedReader().readText().trim()
}.getOrDefault("unknown")

android {
    namespace = "com.enclave.app"
    compileSdk = 36

    defaultConfig {
        // Same applicationId and data directory as the shipping Tauri app so an
        // existing vault is picked up unchanged (`applicationInfo.dataDir`).
        applicationId = "com.enclave.app"
        minSdk = 24
        targetSdk = 36
        // One source of truth for the installed version, so About can show
        // exactly what was installed: apps/android/version.properties, with CI
        // allowed to override it from a tag (-PappVersionName= / -PappVersionCode=).
        versionCode = appVersionCode
        versionName = appVersionName

        ndk {
            // arm64 phones; x86_64 is the emulator target. ONNX Runtime ships no
            // x86_64-android binaries, so offline embeddings stay arm64-only.
            abiFilters += listOf("arm64-v8a", "x86_64")
        }

        // P2P sync runs over ws:// on the LAN and every frame is AEAD-sealed at
        // the application layer, so the platform cleartext flag must be allowed.
        manifestPlaceholders["usesCleartextTraffic"] = "true"

        // Shown in About so a build under test can be identified exactly.
        buildConfigField("String", "GIT_SHA", "\"${gitSha()}\"")
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
        buildConfig = true
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

    // Home-screen widgets (Glance) over the Keystore-wrapped cache.
    implementation("androidx.glance:glance-appwidget:1.1.1")

    debugImplementation("androidx.compose.ui:ui-tooling")

    testImplementation("junit:junit:4.13.2")
    // The mockable android.jar ships org.json stubs that throw "Stub!"; the
    // real implementation is used so the editor model can be tested on the JVM.
    testImplementation("org.json:json:20250107")
}
