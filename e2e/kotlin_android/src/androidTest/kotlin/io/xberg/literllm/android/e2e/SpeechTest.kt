package io.xberg.literllm.android.e2e

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
    }

    @Test
    fun test_edge_speech_long_input() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: edge_speech_long_input */)
    }

    @Test
    fun test_error_speech_auth_401() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: error_speech_auth_401 */)
    }

    @Test
    fun test_error_speech_bad_model() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: error_speech_bad_model */)
    }

    @Test
    fun test_smoke_speech_basic() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: smoke_speech_basic */)
    }

    @Test
    fun test_smoke_speech_mp3_format() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: smoke_speech_mp3_format */)
    }

}
