package dev.kreuzberg.literllm.android

/**
 * JNI entry point for liter-llm on Android.
 *
 * Loads the native liter-llm-ffi library so consumers can drive it via JNA
 * (the same surface as the JVM `dev.kreuzberg.literllm` package) without
 * having to bundle libliter_llm_ffi.so manually. The actual API surface is
 * the JNA-facade Kotlin classes published from `packages/kotlin`; this
 * Android library wraps the prebuilt shared object into an aar consumable
 * from an Android Studio project.
 */
object LiterLlmAndroid {
    init {
        System.loadLibrary("liter_llm_ffi")
    }
}
