package io.xberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ToolCallingTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("liter_llm_jni")
        }
    }

    @Test
    fun test_anthropic_tool_calling() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: anthropic_tool_calling */)
    }

    @Test
    fun test_single_tool_call() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: single_tool_call */)
    }

}
