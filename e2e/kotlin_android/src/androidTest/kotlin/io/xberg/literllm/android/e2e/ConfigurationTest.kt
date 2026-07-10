package io.xberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ConfigurationTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("liter_llm_jni")
        }
    }

    @Test
    fun test_custom_base_url() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: custom_base_url */)
    }

    @Test
    fun test_extra_headers() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: extra_headers */)
    }

    @Test
    fun test_local_provider_llamacpp() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: local_provider_llamacpp */)
    }

    @Test
    fun test_local_provider_ollama() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: local_provider_ollama */)
    }

    @Test
    fun test_local_provider_vllm() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: local_provider_vllm */)
    }

}
