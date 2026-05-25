package dev.kreuzberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class TypesTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("dev.kreuzberg:liter_llm_android_jni")
        }
    }

    @Test
    fun test_all_message_types() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: all_message_types */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_multimodal_content() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: multimodal_content */)
        // TODO: assert result is not an error
    }

}
