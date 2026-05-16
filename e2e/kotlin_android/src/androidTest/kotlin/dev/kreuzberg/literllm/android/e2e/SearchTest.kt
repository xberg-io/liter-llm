package dev.kreuzberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SearchTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("liter_llm_jni")
        }
    }

    @Test
    fun test_search_basic() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: search_basic */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_search_empty_results() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: search_empty_results */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_search_error_400() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: search_error_400 */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_search_error_401() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: search_error_401 */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_search_with_max_results() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: search_with_max_results */)
        // TODO: assert result is not an error
    }

}
