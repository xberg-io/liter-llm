package io.xberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ListModelsTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("liter_llm_jni")
        }
    }

    @Test
    fun test_empty_model_list() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: empty_model_list */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_list_models_error_401() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: list_models_error_401 */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_list_models_error_500() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: list_models_error_500 */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_list_models_filtered() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: list_models_filtered */)
        // TODO: assert result is not an error
    }

}
