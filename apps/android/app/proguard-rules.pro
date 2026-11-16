# Keep the native bridge surface JNA and UniFFI depend on. R8 stripping
# com.sun.jna.Native.fromNative killed every UniFFI call at launch in v1.10.0
# (fixed in v1.10.1) — keep the whole surface here so it cannot regress.
-keep class com.sun.jna.** { *; }
-keep class uniffi.** { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}

# JNA references desktop AWT classes that do not exist on Android. R8 must not
# fail the release build for those missing references; the native app never
# uses the AWT path. (The retired Tauri Android shell carried the same rule.)
-dontwarn java.awt.**
