package io.xberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class OcrTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("liter_llm_jni")
        }
    }

    @Test
    fun test_ocr_error_400() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: ocr_error_400 */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_ocr_error_401() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: ocr_error_401 */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_ocr_multi_page() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: ocr_multi_page */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_ocr_url_document() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: ocr_url_document */)
        // TODO: assert result is not an error
    }

}
