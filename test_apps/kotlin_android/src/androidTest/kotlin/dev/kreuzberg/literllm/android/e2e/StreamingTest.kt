package dev.kreuzberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class StreamingTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("dev.kreuzberg:liter_llm_android_jni")
        }
    }

    @Test
    fun test_anthropic_stream() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: anthropic_stream */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_azure_stream() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: azure_stream */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_basic_stream() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: basic_stream */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_bedrock_stream() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: bedrock_stream */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_edge_stream_function_call() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: edge_stream_function_call */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_empty_stream() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: empty_stream */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_local_stream_ollama() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: local_stream_ollama */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_stream_content_policy_error() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: stream_content_policy_error */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_stream_done_signal() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: stream_done_signal */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_stream_error_401() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: stream_error_401 */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_stream_multiple_choices() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: stream_multiple_choices */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_stream_with_tool_calls() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: stream_with_tool_calls */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_stream_with_usage() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: stream_with_usage */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_vertex_stream() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: vertex_stream */)
        // TODO: assert result is not an error
    }

}
