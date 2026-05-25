package dev.kreuzberg.literllm.android.e2e

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
            System.loadLibrary("dev.kreuzberg:liter_llm_android_jni")
        }
    }

    @Test
    fun test_provider_auth() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: provider_auth */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_register_provider() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: register_provider */)
        // TODO: assert result is not an error
    }

}
