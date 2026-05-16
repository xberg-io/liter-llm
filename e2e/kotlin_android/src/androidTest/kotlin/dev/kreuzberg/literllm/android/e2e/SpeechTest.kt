package dev.kreuzberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SpeechTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("liter_llm_jni")
        }
    }

    @Test
    fun test_edge_speech_all_voices() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: edge_speech_all_voices */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_edge_speech_long_input() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: edge_speech_long_input */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_error_speech_auth_401() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: error_speech_auth_401 */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_error_speech_bad_model() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: error_speech_bad_model */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_smoke_speech_basic() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: smoke_speech_basic */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_smoke_speech_mp3_format() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: smoke_speech_mp3_format */)
        // TODO: assert result is not an error
    }

}
