package io.xberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ResponsesTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("liter_llm_jni")
        }
    }

    @Test
    fun test_edge_response_empty_output() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: edge_response_empty_output */)
    }

    @Test
    fun test_edge_response_large_input() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: edge_response_large_input */)
    }

    @Test
    fun test_error_response_auth_401() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: error_response_auth_401 */)
    }

    @Test
    fun test_error_response_bad_request() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: error_response_bad_request */)
    }

    @Test
    fun test_error_response_not_found() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: error_response_not_found */)
    }

    @Test
    fun test_smoke_cancel_response() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: smoke_cancel_response */)
    }

    @Test
    fun test_smoke_create_response() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: smoke_create_response */)
    }

    @Test
    fun test_smoke_response_with_tools() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: smoke_response_with_tools */)
    }

    @Test
    fun test_smoke_retrieve_response() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: smoke_retrieve_response */)
    }

}
