package io.xberg.literllm.android.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ContractTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("liter_llm_jni")
        }
    }

    @Test
    fun test_binding_api_parity() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: binding_api_parity */)
    }

    @Test
    fun test_contract_ocr() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: contract_ocr */)
    }

    @Test
    fun test_contract_search() {
        val client = LiterLlm()
        val result = client.chat(/* fixture: contract_search */)
    }

}
