package dev.kreuzberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ParityTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("dev.kreuzberg:liter_llm_android_jni")
        }
    }

    @Test
    fun test_api_surface_parity() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: api_surface_parity */)
        // TODO: assert result is not an error
    }

}
