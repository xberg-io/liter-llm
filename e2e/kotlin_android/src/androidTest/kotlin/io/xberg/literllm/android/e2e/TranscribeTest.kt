package io.xberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class TranscribeTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("liter_llm_jni")
        }
    }

    @Test
    fun test_edge_transcribe_empty_audio() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: edge_transcribe_empty_audio */)
    }

    @Test
    fun test_edge_transcribe_with_timestamps() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: edge_transcribe_with_timestamps */)
    }

    @Test
    fun test_error_transcribe_auth_401() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: error_transcribe_auth_401 */)
    }

    @Test
    fun test_error_transcribe_bad_format() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: error_transcribe_bad_format */)
    }

    @Test
    fun test_smoke_transcribe_basic() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: smoke_transcribe_basic */)
    }

    @Test
    fun test_smoke_transcribe_with_language() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: smoke_transcribe_with_language */)
    }

}
