package io.xberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class CustomProviderTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("liter_llm_jni")
        }
    }

    @Test
    fun test_provider_auth() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: provider_auth */)
    }

    @Test
    fun test_register_provider() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: register_provider */)
    }

}
