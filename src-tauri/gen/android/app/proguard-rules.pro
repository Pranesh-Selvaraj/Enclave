# Enclave — Android R8/ProGuard rules.
#
# The generated UniFFI Kotlin bindings (package `uniffi.*`) talk to the Rust
# core through JNA, which resolves each native function by the *Java method
# name* at runtime. R8 renaming those methods (or the classes JNA inspects)
# makes every call fail with UnsatisfiedLinkError on launch — keep the whole
# bridge untouched. JNA itself loads classes reflectively; keep it too.

-keep class uniffi.** { *; }
-keepclassmembers class uniffi.** { *; }

-keep class com.sun.jna.** { *; }
-keepclassmembers class com.sun.jna.** { *; }
-dontwarn java.awt.**
