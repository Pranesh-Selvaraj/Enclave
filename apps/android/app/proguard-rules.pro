# Keep the native bridge surface JNA and UniFFI depend on. R8 stripping
# com.sun.jna.Native.fromNative killed every UniFFI call at launch in v1.10.0
# (fixed in v1.10.1) — keep the whole surface here so it cannot regress.
-keep class com.sun.jna.** { *; }
-keep class uniffi.** { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}
