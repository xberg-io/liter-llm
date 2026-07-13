require "json"

# Low-level binding to the generated C FFI layer (liter_llm.h).
#
# Every non-scalar value crosses the C ABI as a NUL-terminated JSON string
# (`LibC::Char*`); scalars pass by value. Strings returned by the library are
# owned by Rust and must be released with `literllm_free_string`.
#
# Link against the FFI shared library. The library must be installed to a
# standard path, or you can pass --link-flags at build time:
#   crystal build ... --link-flags="-L/path/to/lib -Wl,-rpath,/path/to/lib"
@[Link(ldflags: "-lliter_llm_ffi")]
lib LibLiterllm
  fun free_string = literllm_free_string(ptr : LibC::Char*) : Void
  fun last_error_code = literllm_last_error_code() : Int32
  fun last_error_context = literllm_last_error_context() : LibC::Char*

  struct AudioContent
    _data : Void*
  end
  struct BatchListQuery
    _data : Void*
  end
  struct BatchListResponse
    _data : Void*
  end
  struct BatchObject
    _data : Void*
  end
  struct BudgetConfig
    _data : Void*
  end
  struct CacheConfig
    _data : Void*
  end
  struct ChatCompletionChunk
    _data : Void*
  end
  struct ChatCompletionRequest
    _data : Void*
  end
  struct ChatCompletionResponse
    _data : Void*
  end
  struct CreateBatchRequest
    _data : Void*
  end
  struct CreateFileRequest
    _data : Void*
  end
  struct CreateImageRequest
    _data : Void*
  end
  struct CreateResponseRequest
    _data : Void*
  end
  struct CreateSpeechRequest
    _data : Void*
  end
  struct CreateTranscriptionRequest
    _data : Void*
  end
  struct CustomProviderConfig
    _data : Void*
  end
  struct DecodedDataUrl
    _data : Void*
  end
  struct DeleteResponse
    _data : Void*
  end
  struct EmbeddingRequest
    _data : Void*
  end
  struct EmbeddingResponse
    _data : Void*
  end
  struct FileListQuery
    _data : Void*
  end
  struct FileListResponse
    _data : Void*
  end
  struct FileObject
    _data : Void*
  end
  struct HealthStatus
    _data : Void*
  end
  struct ImageUrl
    _data : Void*
  end
  struct ImagesResponse
    _data : Void*
  end
  struct ModelsListResponse
    _data : Void*
  end
  struct ModerationRequest
    _data : Void*
  end
  struct ModerationResponse
    _data : Void*
  end
  struct OcrRequest
    _data : Void*
  end
  struct OcrResponse
    _data : Void*
  end
  struct ProviderCapabilities
    _data : Void*
  end
  struct ProviderConfig
    _data : Void*
  end
  struct RateLimitConfig
    _data : Void*
  end
  struct RerankRequest
    _data : Void*
  end
  struct RerankResponse
    _data : Void*
  end
  struct ResponseObject
    _data : Void*
  end
  struct SearchRequest
    _data : Void*
  end
  struct SearchResponse
    _data : Void*
  end
  struct TranscriptionResponse
    _data : Void*
  end
  struct WaitForBatchConfig
    _data : Void*
  end
  fun audio_content_from_json = literllm_audio_content_from_json(json : LibC::Char*) : AudioContent*
  fun audio_content_to_json = literllm_audio_content_to_json(ptr : AudioContent*) : LibC::Char*
  fun audio_content_free = literllm_audio_content_free(ptr : AudioContent*)
  fun batch_list_query_from_json = literllm_batch_list_query_from_json(json : LibC::Char*) : BatchListQuery*
  fun batch_list_query_to_json = literllm_batch_list_query_to_json(ptr : BatchListQuery*) : LibC::Char*
  fun batch_list_query_free = literllm_batch_list_query_free(ptr : BatchListQuery*)
  fun batch_list_response_from_json = literllm_batch_list_response_from_json(json : LibC::Char*) : BatchListResponse*
  fun batch_list_response_to_json = literllm_batch_list_response_to_json(ptr : BatchListResponse*) : LibC::Char*
  fun batch_list_response_free = literllm_batch_list_response_free(ptr : BatchListResponse*)
  fun batch_object_from_json = literllm_batch_object_from_json(json : LibC::Char*) : BatchObject*
  fun batch_object_to_json = literllm_batch_object_to_json(ptr : BatchObject*) : LibC::Char*
  fun batch_object_free = literllm_batch_object_free(ptr : BatchObject*)
  fun budget_config_from_json = literllm_budget_config_from_json(json : LibC::Char*) : BudgetConfig*
  fun budget_config_to_json = literllm_budget_config_to_json(ptr : BudgetConfig*) : LibC::Char*
  fun budget_config_free = literllm_budget_config_free(ptr : BudgetConfig*)
  fun cache_config_from_json = literllm_cache_config_from_json(json : LibC::Char*) : CacheConfig*
  fun cache_config_to_json = literllm_cache_config_to_json(ptr : CacheConfig*) : LibC::Char*
  fun cache_config_free = literllm_cache_config_free(ptr : CacheConfig*)
  fun chat_completion_chunk_from_json = literllm_chat_completion_chunk_from_json(json : LibC::Char*) : ChatCompletionChunk*
  fun chat_completion_chunk_to_json = literllm_chat_completion_chunk_to_json(ptr : ChatCompletionChunk*) : LibC::Char*
  fun chat_completion_chunk_free = literllm_chat_completion_chunk_free(ptr : ChatCompletionChunk*)
  fun chat_completion_request_from_json = literllm_chat_completion_request_from_json(json : LibC::Char*) : ChatCompletionRequest*
  fun chat_completion_request_to_json = literllm_chat_completion_request_to_json(ptr : ChatCompletionRequest*) : LibC::Char*
  fun chat_completion_request_free = literllm_chat_completion_request_free(ptr : ChatCompletionRequest*)
  fun chat_completion_response_from_json = literllm_chat_completion_response_from_json(json : LibC::Char*) : ChatCompletionResponse*
  fun chat_completion_response_to_json = literllm_chat_completion_response_to_json(ptr : ChatCompletionResponse*) : LibC::Char*
  fun chat_completion_response_free = literllm_chat_completion_response_free(ptr : ChatCompletionResponse*)
  fun create_batch_request_from_json = literllm_create_batch_request_from_json(json : LibC::Char*) : CreateBatchRequest*
  fun create_batch_request_to_json = literllm_create_batch_request_to_json(ptr : CreateBatchRequest*) : LibC::Char*
  fun create_batch_request_free = literllm_create_batch_request_free(ptr : CreateBatchRequest*)
  fun create_file_request_from_json = literllm_create_file_request_from_json(json : LibC::Char*) : CreateFileRequest*
  fun create_file_request_to_json = literllm_create_file_request_to_json(ptr : CreateFileRequest*) : LibC::Char*
  fun create_file_request_free = literllm_create_file_request_free(ptr : CreateFileRequest*)
  fun create_image_request_from_json = literllm_create_image_request_from_json(json : LibC::Char*) : CreateImageRequest*
  fun create_image_request_to_json = literllm_create_image_request_to_json(ptr : CreateImageRequest*) : LibC::Char*
  fun create_image_request_free = literllm_create_image_request_free(ptr : CreateImageRequest*)
  fun create_response_request_from_json = literllm_create_response_request_from_json(json : LibC::Char*) : CreateResponseRequest*
  fun create_response_request_to_json = literllm_create_response_request_to_json(ptr : CreateResponseRequest*) : LibC::Char*
  fun create_response_request_free = literllm_create_response_request_free(ptr : CreateResponseRequest*)
  fun create_speech_request_from_json = literllm_create_speech_request_from_json(json : LibC::Char*) : CreateSpeechRequest*
  fun create_speech_request_to_json = literllm_create_speech_request_to_json(ptr : CreateSpeechRequest*) : LibC::Char*
  fun create_speech_request_free = literllm_create_speech_request_free(ptr : CreateSpeechRequest*)
  fun create_transcription_request_from_json = literllm_create_transcription_request_from_json(json : LibC::Char*) : CreateTranscriptionRequest*
  fun create_transcription_request_to_json = literllm_create_transcription_request_to_json(ptr : CreateTranscriptionRequest*) : LibC::Char*
  fun create_transcription_request_free = literllm_create_transcription_request_free(ptr : CreateTranscriptionRequest*)
  fun custom_provider_config_from_json = literllm_custom_provider_config_from_json(json : LibC::Char*) : CustomProviderConfig*
  fun custom_provider_config_to_json = literllm_custom_provider_config_to_json(ptr : CustomProviderConfig*) : LibC::Char*
  fun custom_provider_config_free = literllm_custom_provider_config_free(ptr : CustomProviderConfig*)
  fun decoded_data_url_from_json = literllm_decoded_data_url_from_json(json : LibC::Char*) : DecodedDataUrl*
  fun decoded_data_url_to_json = literllm_decoded_data_url_to_json(ptr : DecodedDataUrl*) : LibC::Char*
  fun decoded_data_url_free = literllm_decoded_data_url_free(ptr : DecodedDataUrl*)
  fun delete_response_from_json = literllm_delete_response_from_json(json : LibC::Char*) : DeleteResponse*
  fun delete_response_to_json = literllm_delete_response_to_json(ptr : DeleteResponse*) : LibC::Char*
  fun delete_response_free = literllm_delete_response_free(ptr : DeleteResponse*)
  fun embedding_request_from_json = literllm_embedding_request_from_json(json : LibC::Char*) : EmbeddingRequest*
  fun embedding_request_to_json = literllm_embedding_request_to_json(ptr : EmbeddingRequest*) : LibC::Char*
  fun embedding_request_free = literllm_embedding_request_free(ptr : EmbeddingRequest*)
  fun embedding_response_from_json = literllm_embedding_response_from_json(json : LibC::Char*) : EmbeddingResponse*
  fun embedding_response_to_json = literllm_embedding_response_to_json(ptr : EmbeddingResponse*) : LibC::Char*
  fun embedding_response_free = literllm_embedding_response_free(ptr : EmbeddingResponse*)
  fun file_list_query_from_json = literllm_file_list_query_from_json(json : LibC::Char*) : FileListQuery*
  fun file_list_query_to_json = literllm_file_list_query_to_json(ptr : FileListQuery*) : LibC::Char*
  fun file_list_query_free = literllm_file_list_query_free(ptr : FileListQuery*)
  fun file_list_response_from_json = literllm_file_list_response_from_json(json : LibC::Char*) : FileListResponse*
  fun file_list_response_to_json = literllm_file_list_response_to_json(ptr : FileListResponse*) : LibC::Char*
  fun file_list_response_free = literllm_file_list_response_free(ptr : FileListResponse*)
  fun file_object_from_json = literllm_file_object_from_json(json : LibC::Char*) : FileObject*
  fun file_object_to_json = literllm_file_object_to_json(ptr : FileObject*) : LibC::Char*
  fun file_object_free = literllm_file_object_free(ptr : FileObject*)
  fun health_status_from_json = literllm_health_status_from_json(json : LibC::Char*) : HealthStatus*
  fun health_status_to_json = literllm_health_status_to_json(ptr : HealthStatus*) : LibC::Char*
  fun health_status_free = literllm_health_status_free(ptr : HealthStatus*)
  fun image_url_from_json = literllm_image_url_from_json(json : LibC::Char*) : ImageUrl*
  fun image_url_to_json = literllm_image_url_to_json(ptr : ImageUrl*) : LibC::Char*
  fun image_url_free = literllm_image_url_free(ptr : ImageUrl*)
  fun images_response_from_json = literllm_images_response_from_json(json : LibC::Char*) : ImagesResponse*
  fun images_response_to_json = literllm_images_response_to_json(ptr : ImagesResponse*) : LibC::Char*
  fun images_response_free = literllm_images_response_free(ptr : ImagesResponse*)
  fun models_list_response_from_json = literllm_models_list_response_from_json(json : LibC::Char*) : ModelsListResponse*
  fun models_list_response_to_json = literllm_models_list_response_to_json(ptr : ModelsListResponse*) : LibC::Char*
  fun models_list_response_free = literllm_models_list_response_free(ptr : ModelsListResponse*)
  fun moderation_request_from_json = literllm_moderation_request_from_json(json : LibC::Char*) : ModerationRequest*
  fun moderation_request_to_json = literllm_moderation_request_to_json(ptr : ModerationRequest*) : LibC::Char*
  fun moderation_request_free = literllm_moderation_request_free(ptr : ModerationRequest*)
  fun moderation_response_from_json = literllm_moderation_response_from_json(json : LibC::Char*) : ModerationResponse*
  fun moderation_response_to_json = literllm_moderation_response_to_json(ptr : ModerationResponse*) : LibC::Char*
  fun moderation_response_free = literllm_moderation_response_free(ptr : ModerationResponse*)
  fun ocr_request_from_json = literllm_ocr_request_from_json(json : LibC::Char*) : OcrRequest*
  fun ocr_request_to_json = literllm_ocr_request_to_json(ptr : OcrRequest*) : LibC::Char*
  fun ocr_request_free = literllm_ocr_request_free(ptr : OcrRequest*)
  fun ocr_response_from_json = literllm_ocr_response_from_json(json : LibC::Char*) : OcrResponse*
  fun ocr_response_to_json = literllm_ocr_response_to_json(ptr : OcrResponse*) : LibC::Char*
  fun ocr_response_free = literllm_ocr_response_free(ptr : OcrResponse*)
  fun provider_capabilities_from_json = literllm_provider_capabilities_from_json(json : LibC::Char*) : ProviderCapabilities*
  fun provider_capabilities_to_json = literllm_provider_capabilities_to_json(ptr : ProviderCapabilities*) : LibC::Char*
  fun provider_capabilities_free = literllm_provider_capabilities_free(ptr : ProviderCapabilities*)
  fun provider_config_from_json = literllm_provider_config_from_json(json : LibC::Char*) : ProviderConfig*
  fun provider_config_to_json = literllm_provider_config_to_json(ptr : ProviderConfig*) : LibC::Char*
  fun provider_config_free = literllm_provider_config_free(ptr : ProviderConfig*)
  fun rate_limit_config_from_json = literllm_rate_limit_config_from_json(json : LibC::Char*) : RateLimitConfig*
  fun rate_limit_config_to_json = literllm_rate_limit_config_to_json(ptr : RateLimitConfig*) : LibC::Char*
  fun rate_limit_config_free = literllm_rate_limit_config_free(ptr : RateLimitConfig*)
  fun rerank_request_from_json = literllm_rerank_request_from_json(json : LibC::Char*) : RerankRequest*
  fun rerank_request_to_json = literllm_rerank_request_to_json(ptr : RerankRequest*) : LibC::Char*
  fun rerank_request_free = literllm_rerank_request_free(ptr : RerankRequest*)
  fun rerank_response_from_json = literllm_rerank_response_from_json(json : LibC::Char*) : RerankResponse*
  fun rerank_response_to_json = literllm_rerank_response_to_json(ptr : RerankResponse*) : LibC::Char*
  fun rerank_response_free = literllm_rerank_response_free(ptr : RerankResponse*)
  fun response_object_from_json = literllm_response_object_from_json(json : LibC::Char*) : ResponseObject*
  fun response_object_to_json = literllm_response_object_to_json(ptr : ResponseObject*) : LibC::Char*
  fun response_object_free = literllm_response_object_free(ptr : ResponseObject*)
  fun search_request_from_json = literllm_search_request_from_json(json : LibC::Char*) : SearchRequest*
  fun search_request_to_json = literllm_search_request_to_json(ptr : SearchRequest*) : LibC::Char*
  fun search_request_free = literllm_search_request_free(ptr : SearchRequest*)
  fun search_response_from_json = literllm_search_response_from_json(json : LibC::Char*) : SearchResponse*
  fun search_response_to_json = literllm_search_response_to_json(ptr : SearchResponse*) : LibC::Char*
  fun search_response_free = literllm_search_response_free(ptr : SearchResponse*)
  fun transcription_response_from_json = literllm_transcription_response_from_json(json : LibC::Char*) : TranscriptionResponse*
  fun transcription_response_to_json = literllm_transcription_response_to_json(ptr : TranscriptionResponse*) : LibC::Char*
  fun transcription_response_free = literllm_transcription_response_free(ptr : TranscriptionResponse*)
  fun wait_for_batch_config_from_json = literllm_wait_for_batch_config_from_json(json : LibC::Char*) : WaitForBatchConfig*
  fun wait_for_batch_config_to_json = literllm_wait_for_batch_config_to_json(ptr : WaitForBatchConfig*) : LibC::Char*
  fun wait_for_batch_config_free = literllm_wait_for_batch_config_free(ptr : WaitForBatchConfig*)

  # Create a new LLM client with simple scalar configuration.
  fun create_client = literllm_create_client(api_key : LibC::Char*, base_url : LibC::Char*, timeout_secs : UInt64, max_retries : UInt32, model_hint : LibC::Char*) : Void*
  # Create a new LLM client from a JSON string.
  fun create_client_from_json = literllm_create_client_from_json(json : LibC::Char*) : Void*
  # Encode bytes as a base64 data URL: `data:<mime>;base64,<b64>`.
  fun encode_data_url = literllm_encode_data_url(bytes : LibC::Char*, mime : LibC::Char*) : LibC::Char*
  # Decode a base64 data URL into [`DecodedDataUrl`].
  fun decode_data_url = literllm_decode_data_url(url : LibC::Char*) : DecodedDataUrl*
  # Register a custom provider in the global runtime registry.
  fun register_custom_provider = literllm_register_custom_provider(config : CustomProviderConfig*) : Void
  # Remove a previously registered custom provider by name.
  fun unregister_custom_provider = literllm_unregister_custom_provider(name : LibC::Char*) : Bool
  # Return the capability flags for a named provider.
  fun capabilities = literllm_capabilities(provider_name : LibC::Char*) : ProviderCapabilities*
  # Return all provider configs from the registry.
  fun all_providers = literllm_all_providers() : LibC::Char*
  # Return the set of complex provider names.
  fun complex_provider_names = literllm_complex_provider_names() : LibC::Char*
  # Calculate the estimated cost of a completion given a model name and token
  fun completion_cost = literllm_completion_cost(model : LibC::Char*, prompt_tokens : UInt64, completion_tokens : UInt64) : LibC::Char*
  # Calculate the estimated cost of a completion, accounting for cached
  fun completion_cost_with_cache = literllm_completion_cost_with_cache(model : LibC::Char*, prompt_tokens : UInt64, cached_tokens : UInt64, completion_tokens : UInt64) : LibC::Char*
  # Remove all guardrails from the global registry.
  fun clear = literllm_clear() : Void
  # Count tokens in a text string using the tokenizer for the given model.
  fun count_tokens = literllm_count_tokens(model : LibC::Char*, text : LibC::Char*) : LibC::SizeT
  # Count tokens for a full [`ChatCompletionRequest`].
  fun count_request_tokens = literllm_count_request_tokens(model : LibC::Char*, req : ChatCompletionRequest*) : LibC::SizeT
  # Assert that `current_len + incoming` does not exceed `limit`.
  fun check_bound = literllm_check_bound(context : LibC::Char*, current_len : LibC::SizeT, incoming : LibC::SizeT, limit : LibC::SizeT) : Void
  # Install the `ring` crypto provider as the rustls process default, idempotently.
  fun ensure_crypto_provider = literllm_ensure_crypto_provider() : Void
  fun chunk_middleware_process = literllm_chunk_middleware_process(handle : Void*, chunk : ChatCompletionChunk*) : LibC::Char*
  fun chunk_middleware_free = literllm_chunk_middleware_free(handle : Void*) : Void
  fun default_client_fetch_batch_for_polling = literllm_default_client_fetch_batch_for_polling(handle : Void*, batch_id : LibC::Char*) : BatchObject*
  fun default_client_wait_for_batch = literllm_default_client_wait_for_batch(handle : Void*, batch_id : LibC::Char*, config : WaitForBatchConfig*) : BatchObject*
  fun default_client_free = literllm_default_client_free(handle : Void*) : Void
  fun health_checker_check = literllm_health_checker_check(handle : Void*, upstream : LibC::Char*) : HealthStatus*
  fun health_checker_free = literllm_health_checker_free(handle : Void*) : Void
  fun singleflight_result_free = literllm_singleflight_result_free(handle : Void*) : Void
  fun default_client_chat_stream_start = literllm_default_client_chat_stream_start(handle : Void*, req : ChatCompletionRequest*) : Void*
  fun default_client_chat_stream_next = literllm_default_client_chat_stream_next(handle : Void*) : Void*
  fun default_client_chat_stream_free = literllm_default_client_chat_stream_free(handle : Void*) : Void
  fun default_client_chat_json = literllm_default_client_chat_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_embed_json = literllm_default_client_embed_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_list_models_json = literllm_default_client_list_models_json(handle : Void*) : LibC::Char*
  fun default_client_image_generate_json = literllm_default_client_image_generate_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_transcribe_json = literllm_default_client_transcribe_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_moderate_json = literllm_default_client_moderate_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_rerank_json = literllm_default_client_rerank_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_search_json = literllm_default_client_search_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_speech_json = literllm_default_client_speech_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_ocr_json = literllm_default_client_ocr_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_create_file_json = literllm_default_client_create_file_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_retrieve_file_json = literllm_default_client_retrieve_file_json(handle : Void*, file_id : LibC::Char*) : LibC::Char*
  fun default_client_delete_file_json = literllm_default_client_delete_file_json(handle : Void*, file_id : LibC::Char*) : LibC::Char*
  fun default_client_list_files_json = literllm_default_client_list_files_json(handle : Void*, query_json : LibC::Char*) : LibC::Char*
  fun default_client_file_content_json = literllm_default_client_file_content_json(handle : Void*, file_id : LibC::Char*) : LibC::Char*
  fun default_client_create_batch_json = literllm_default_client_create_batch_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_retrieve_batch_json = literllm_default_client_retrieve_batch_json(handle : Void*, batch_id : LibC::Char*) : LibC::Char*
  fun default_client_list_batches_json = literllm_default_client_list_batches_json(handle : Void*, query_json : LibC::Char*) : LibC::Char*
  fun default_client_cancel_batch_json = literllm_default_client_cancel_batch_json(handle : Void*, batch_id : LibC::Char*) : LibC::Char*
  fun default_client_create_response_json = literllm_default_client_create_response_json(handle : Void*, req_json : LibC::Char*) : LibC::Char*
  fun default_client_retrieve_response_json = literllm_default_client_retrieve_response_json(handle : Void*, response_id : LibC::Char*) : LibC::Char*
  fun default_client_cancel_response_json = literllm_default_client_cancel_response_json(handle : Void*, response_id : LibC::Char*) : LibC::Char*
end

# liter-llm — Crystal bindings generated by alef.
#
# Ruby-style API over the Rust core: snake_case methods, PascalCase types,
# Rust-like generic containers (`Array(T)`, `Hash(K, V)`), and fiber/`Channel`
# based concurrency for async and streaming methods.
module LiterLlm
  VERSION = "1.9.3"

  # System message guiding model behavior for the entire conversation.
  class SystemMessage
    include JSON::Serializable
    # Instructions or context that apply throughout the conversation.
    #
    # Accepts either a plain text string or an array of content parts,
    # mirroring [`UserContent`] so that `Message::system_with_parts` works.
    getter content : UserContent = UserContent.from_json("{}")
    # Optional name for the system message source.
    getter name : String?
  end

  # User message in the conversation.
  class UserMessage
    include JSON::Serializable
    # Message content as plain text or array of content parts (text, images, documents, audio).
    getter content : UserContent = UserContent.from_json("{}")
    # Optional name for the user.
    getter name : String?
  end

  # An image URL reference with optional detail level for processing.
  class ImageUrl
    include JSON::Serializable
    # URL of the image (data URI or HTTP/HTTPS URL).
    getter url : String = ""
    # Detail level: low (512x512), high (2x2 tiles), or auto (model-selected).
    getter detail : ImageDetail?
  end

  # PDF/document content part for vision-capable models.
  class DocumentContent
    include JSON::Serializable
    # Base64-encoded document data or URL.
    getter data : String = ""
    # MIME type (e.g., "application/pdf", "text/csv").
    getter media_type : String = ""
  end

  # Audio content part for speech-capable models.
  class AudioContent
    include JSON::Serializable
    # Base64-encoded audio data.
    getter data : String = ""
    # Audio format (e.g., "wav", "mp3", "ogg").
    getter format : String = ""
  end

  # Assistant's response to a user message.
  class AssistantMessage
    include JSON::Serializable
    # The assistant's response: plain text, structured parts, or absent.
    #
    # `None` is valid when the model replies with tool calls only.
    getter content : AssistantContent?
    # Optional name for the assistant.
    getter name : String?
    # Tool calls the model wants to execute, if any.
    getter tool_calls : Array(ToolCall)?
    # Refusal reason, if the model declined to respond per safety policies.
    getter refusal : String?
    # Deprecated legacy function_call field; retained for API compatibility.
    getter function_call : FunctionCall?
  end

  # Tool execution result returned to the model.
  class ToolMessage
    include JSON::Serializable
    # Result of the tool execution.
    getter content : String = ""
    # ID of the tool call this result responds to.
    getter tool_call_id : String = ""
    # Optional tool/function name.
    getter name : String?
  end

  # Developer message (system-like message for Claude models).
  class DeveloperMessage
    include JSON::Serializable
    # Developer-specific instructions or context.
    getter content : String = ""
    # Optional name for the developer message source.
    getter name : String?
  end

  # Deprecated legacy function-role message body.
  class FunctionMessage
    include JSON::Serializable
    getter content : String = ""
    getter name : String = ""
  end

  # A tool the model can invoke (currently, all tools are functions).
  class ChatCompletionTool
    include JSON::Serializable
    # Tool type (always "function" in OpenAI spec).
    @[JSON::Field(key: "type")]
    getter tool_type : ToolType = ToolType::Function
    # Function definition with name, description, and JSON schema parameters.
    getter function : FunctionDefinition = FunctionDefinition.from_json("{}")
  end

  # Function definition exposed to the model.
  class FunctionDefinition
    include JSON::Serializable
    # Name of the function. Required and must be alphanumeric + underscores.
    getter name : String = ""
    # Human-readable description explaining what the function does.
    getter description : String?
    # JSON Schema defining the function's parameters.
    getter parameters : JSON::Any?
    # If true, enforce strict JSON schema validation for arguments.
    getter strict : Bool?
  end

  # A tool call the model wants to execute.
  class ToolCall
    include JSON::Serializable
    # Unique ID for this call, used to reference in tool result messages.
    getter id : String = ""
    # Tool type (always "function").
    @[JSON::Field(key: "type")]
    getter call_type : ToolType = ToolType::Function
    # Function name and arguments.
    getter function : FunctionCall = FunctionCall.from_json("{}")
  end

  # Function call details.
  class FunctionCall
    include JSON::Serializable
    # Function name.
    getter name : String = ""
    # Arguments as a JSON string (parse with serde_json::from_str).
    getter arguments : String = ""
  end

  # Directive to call a specific tool.
  class SpecificToolChoice
    include JSON::Serializable
    # Tool type (always "function").
    @[JSON::Field(key: "type")]
    getter choice_type : ToolType = ToolType::Function
    # The specific function to invoke.
    getter function : SpecificFunction = SpecificFunction.from_json("{}")
  end

  # Name of the specific function to invoke.
  class SpecificFunction
    include JSON::Serializable
    # Function name.
    getter name : String = ""
  end

  # JSON Schema specification for constrained output.
  class JsonSchemaFormat
    include JSON::Serializable
    # Name of the schema (must be unique in the request).
    getter name : String = ""
    # Description of what the schema represents.
    getter description : String?
    # JSON Schema object defining the output structure.
    getter schema : JSON::Any = JSON::Any.new(nil)
    # If true, enforce strict schema validation.
    getter strict : Bool?
  end

  # Token-usage accounting returned by the provider on each completion / embedding call.
  class Usage
    include JSON::Serializable
    # Prompt tokens used. Defaults to 0 when absent (some providers omit this).
    getter prompt_tokens : UInt64 = 0
    # Completion tokens used. Defaults to 0 when absent (e.g. embedding responses).
    getter completion_tokens : UInt64 = 0
    # Total tokens used. Defaults to 0 when absent (some providers omit this).
    getter total_tokens : UInt64 = 0
    # Breakdown of tokens used in the prompt, including cached tokens served
    # at the provider's discounted cache-read rate. Absent when the provider
    # does not return prompt-token details.
    getter prompt_tokens_details : PromptTokensDetails?
  end

  # Breakdown of tokens used in the prompt portion of a request.
  #
  # `cached_tokens` is included in `Usage::prompt_tokens` — it is *not* an
  # additional charge on top of the prompt token count. When pricing supports
  # a `cache_read_input_token_cost`, the cached portion is billed at the
  # discounted rate and the remainder at the regular input rate.
  class PromptTokensDetails
    include JSON::Serializable
    # Cached tokens present in the prompt. Defaults to 0 when absent.
    getter cached_tokens : UInt64 = 0
    # Audio input tokens present in the prompt. Defaults to 0 when absent.
    getter audio_tokens : UInt64 = 0
  end

  # Chat completion request (compatible with OpenAI and similar APIs).
  class ChatCompletionRequest
    include JSON::Serializable
    # Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`).
    getter model : String = ""
    # Conversation history from oldest to newest.
    getter messages : Array(Message) = [] of Message
    # Sampling temperature in `[0.0, 2.0]`. Higher increases randomness. Defaults to 1.0.
    getter temperature : Float64?
    # Nucleus sampling parameter in `[0.0, 1.0]`. Lower is more focused.
    getter top_p : Float64?
    # Number of chat completions to generate. Defaults to 1.
    getter n : UInt32?
    # Whether to stream the response.
    #
    # Managed by the client layer — do not set directly.
    getter stream : Bool?
    # Stop sequence(s) that halt token generation.
    getter stop : StopSequence?
    # Max output tokens. Different from max_completion_tokens in some providers.
    getter max_tokens : UInt64?
    # Presence penalty in `[-2.0, 2.0]`. Positive discourages repeated topics.
    getter presence_penalty : Float64?
    # Frequency penalty in `[-2.0, 2.0]`. Positive discourages repeated tokens.
    getter frequency_penalty : Float64?
    # Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic
    # serialization order — important when hashing or signing requests.
    getter logit_bias : Hash(String, Float64)?
    # User identifier for request tracking and abuse detection.
    getter user : String?
    # Tools the model can invoke.
    getter tools : Array(ChatCompletionTool)?
    # Tool usage mode (auto, required, none, or specific tool).
    getter tool_choice : ToolChoice?
    # Whether the model can call multiple tools in parallel. Defaults to true.
    getter parallel_tool_calls : Bool?
    # Output format constraint (text, JSON, JSON schema).
    getter response_format : ResponseFormat?
    # Streaming options (e.g., include_usage).
    getter stream_options : StreamOptions?
    # Random seed for reproducible outputs. Provider support varies.
    getter seed : Int64?
    # Reasoning effort level (low, medium, high) for extended-thinking models.
    getter reasoning_effort : ReasoningEffort?
    # Output modalities to request from the model.
    #
    # For OpenAI audio models, pass `["text", "audio"]`. Vertex AI / Gemini
    # translates these to `generationConfig.responseModalities` (uppercase).
    getter modalities : Array(Modality)?
    # Provider-specific extra parameters merged into the request body.
    # Use for guardrails, safety settings, grounding config, etc.
    getter extra_body : JSON::Any?
  end

  # Options for streaming responses.
  class StreamOptions
    include JSON::Serializable
    # If true, include token usage in the final stream chunk.
    getter include_usage : Bool?
  end

  # Chat completion response from the API.
  class ChatCompletionResponse
    include JSON::Serializable
    # Unique identifier for this response.
    getter id : String = ""
    # Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a
    # plain `String` so non-standard provider values do not break deserialization.
    getter object : String = ""
    # Unix timestamp of response creation.
    getter created : UInt64 = 0
    # Model used to generate the response.
    getter model : String = ""
    # List of completion choices.
    getter choices : Array(Choice) = [] of Choice
    # Token usage statistics.
    getter usage : Usage?
    # Fingerprint of the system configuration (OpenAI-specific).
    getter system_fingerprint : String?
    # Service tier used (OpenAI-specific).
    getter service_tier : String?
  end

  # A single completion choice.
  class Choice
    include JSON::Serializable
    # Index of this choice in the choices array.
    getter index : UInt32 = 0
    # The assistant's message response.
    getter message : AssistantMessage = AssistantMessage.from_json("{}")
    # Why the model stopped generating (stop, length, tool_calls, content_filter, etc.).
    getter finish_reason : FinishReason?
  end

  # A streamed chunk of a chat completion response.
  class ChatCompletionChunk
    include JSON::Serializable
    # Unique identifier for this stream.
    getter id : String = ""
    # Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored
    # as a plain `String` so non-standard provider values do not fail parsing.
    getter object : String = ""
    # Unix timestamp of chunk creation.
    getter created : UInt64 = 0
    # Model used to generate the chunk.
    getter model : String = ""
    # Streaming choices (delta updates).
    getter choices : Array(StreamChoice) = [] of StreamChoice
    # Token usage (typically only in the final chunk).
    getter usage : Usage?
    # Fingerprint of the system configuration (OpenAI-specific).
    getter system_fingerprint : String?
    # Service tier used (OpenAI-specific).
    getter service_tier : String?
  end

  # A streaming choice with incremental delta.
  class StreamChoice
    include JSON::Serializable
    # Index of this choice in the choices array.
    getter index : UInt32 = 0
    # Incremental update to the message (content, tool calls, etc.).
    getter delta : StreamDelta = StreamDelta.from_json("{}")
    # Why the stream ended (present only in final chunk).
    getter finish_reason : FinishReason?
  end

  # Incremental delta in a stream chunk.
  class StreamDelta
    include JSON::Serializable
    # Role (typically present only in the first chunk).
    getter role : String?
    # Partial content chunk (e.g., a few words of the response).
    getter content : String?
    # Partial tool calls being streamed.
    getter tool_calls : Array(StreamToolCall)?
    # Deprecated legacy function_call delta; retained for API compatibility.
    getter function_call : StreamFunctionCall?
    # Partial refusal message.
    getter refusal : String?
  end

  # A streaming tool call being built incrementally.
  class StreamToolCall
    include JSON::Serializable
    # Index of this tool call in the tool_calls array.
    getter index : UInt32 = 0
    # Tool call ID (typically in the first chunk for this call).
    getter id : String?
    # Tool type (typically "function").
    @[JSON::Field(key: "type")]
    getter call_type : ToolType?
    # Partial function name and arguments.
    getter function : StreamFunctionCall?
  end

  # Partial function call details in a stream.
  class StreamFunctionCall
    include JSON::Serializable
    # Function name (typically in the first chunk).
    getter name : String?
    # Partial JSON arguments chunk.
    getter arguments : String?
  end

  # Embedding request.
  class EmbeddingRequest
    include JSON::Serializable
    # Model ID (e.g., `"text-embedding-3-small"`).
    getter model : String = ""
    # Text or texts to embed.
    getter input : EmbeddingInput = EmbeddingInput.from_json("{}")
    # Output format: float (native) or base64.
    getter encoding_format : EmbeddingFormat?
    # Requested embedding dimensions (if supported by the model).
    getter dimensions : UInt32?
    # User identifier for request tracking.
    getter user : String?
  end

  # Embedding response.
  class EmbeddingResponse
    include JSON::Serializable
    # Always `"list"` from OpenAI-compatible APIs.  Stored as a plain
    # `String` so non-standard provider values do not break deserialization.
    getter object : String = ""
    # List of embeddings.
    getter data : Array(EmbeddingObject) = [] of EmbeddingObject
    # Model used to generate embeddings.
    getter model : String = ""
    # Token usage (input tokens only; embeddings have zero output tokens).
    getter usage : Usage?
  end

  # A single embedding vector.
  class EmbeddingObject
    include JSON::Serializable
    # Always `"embedding"` from OpenAI-compatible APIs.  Stored as a plain
    # `String` so non-standard provider values do not break deserialization.
    getter object : String = ""
    # The embedding vector.
    getter embedding : Array(Float64) = [] of Float64
    # Index in the batch (corresponds to input order).
    getter index : UInt32 = 0
  end

  # Request to create images from a text prompt.
  class CreateImageRequest
    include JSON::Serializable
    # Text description of the image to generate.
    getter prompt : String = ""
    # Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset.
    getter model : String?
    # Number of images to generate. Defaults to 1.
    getter n : UInt32?
    # Image size (e.g., `"1024x1024"`, `"1792x1024"`).
    getter size : String?
    # Image quality: `"standard"` or `"hd"`.
    getter quality : String?
    # Style: `"natural"` or `"vivid"` (DALL-E 3 only).
    getter style : String?
    # Response format: `"url"` or `"b64_json"`.
    getter response_format : String?
    # User identifier for request tracking.
    getter user : String?
  end

  # Response containing generated images.
  class ImagesResponse
    include JSON::Serializable
    # Unix timestamp of image creation.
    getter created : UInt64 = 0
    # List of generated images.
    getter data : Array(Image) = [] of Image
  end

  # A single generated image, returned as either a URL or base64 data.
  class Image
    include JSON::Serializable
    # Image URL (if response_format was "url").
    getter url : String?
    # Base64-encoded image data (if response_format was "b64_json").
    getter b64_json : String?
    # The final prompt used to generate the image (DALL-E 3).
    getter revised_prompt : String?
  end

  # Result of decoding a `data:` URL — MIME type and the decoded byte payload.
  #
  # Named struct (rather than a tuple) so polyglot bindings can extract
  # `decode_data_url` with a typed return rather than a sanitized scalar.
  class DecodedDataUrl
    include JSON::Serializable
    # MIME type extracted from the URL prefix (verbatim, not normalised).
    getter mime : String = ""
    # Decoded base64 payload.
    @[JSON::Field(ignore: true)]
    getter data : Bytes = Bytes.empty
  end

  # Request to generate speech audio from text.
  class CreateSpeechRequest
    include JSON::Serializable
    # Model ID (e.g., `"tts-1"`, `"tts-1-hd"`).
    getter model : String = ""
    # Text to synthesize into speech.
    getter input : String = ""
    # Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`).
    getter voice : String = ""
    # Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`).
    getter response_format : String?
    # Playback speed in `[0.25, 4.0]`. Defaults to 1.0.
    getter speed : Float64?
  end

  # Request to transcribe audio into text.
  class CreateTranscriptionRequest
    include JSON::Serializable
    # Model ID (e.g., `"whisper-1"`).
    getter model : String = ""
    # Base64-encoded audio file data.
    getter file : String = ""
    # Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects.
    getter language : String?
    # Optional text to guide the model (improves accuracy for domain-specific terms).
    getter prompt : String?
    # Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`).
    getter response_format : String?
    # Sampling temperature in `[0.0, 1.0]`. Higher increases variability. Defaults to 0.
    getter temperature : Float64?
  end

  # Response from a transcription request.
  class TranscriptionResponse
    include JSON::Serializable
    # The transcribed text.
    getter text : String = ""
    # Detected language (ISO-639-1 code).
    getter language : String?
    # Total audio duration in seconds.
    getter duration : Float64?
    # Detailed segment-level transcription (if response_format is "verbose_json").
    getter segments : Array(TranscriptionSegment)?
  end

  # A segment of transcribed audio with timing information.
  class TranscriptionSegment
    include JSON::Serializable
    # Segment index (0-based).
    getter id : UInt32 = 0
    # Start time in seconds.
    getter start : Float64 = 0.0
    # End time in seconds.
    @[JSON::Field(key: "end")]
    getter end_ : Float64 = 0.0
    # Transcribed text for this segment.
    getter text : String = ""
  end

  # Request to classify content for policy violations.
  class ModerationRequest
    include JSON::Serializable
    # Text or texts to check.
    getter input : ModerationInput = ModerationInput.from_json("{}")
    # Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset.
    getter model : String?
  end

  # Response from the moderation endpoint.
  class ModerationResponse
    include JSON::Serializable
    # Unique identifier for this moderation request.
    getter id : String = ""
    # Model used for classification.
    getter model : String = ""
    # Results for each input string.
    getter results : Array(ModerationResult) = [] of ModerationResult
  end

  # A single moderation classification result.
  class ModerationResult
    include JSON::Serializable
    # True if any category was flagged.
    getter flagged : Bool = false
    # Boolean flags for each moderation category.
    getter categories : ModerationCategories = ModerationCategories.from_json("{}")
    # Confidence scores for each category.
    getter category_scores : ModerationCategoryScores = ModerationCategoryScores.from_json("{}")
  end

  # Boolean flags for each moderation category.
  class ModerationCategories
    include JSON::Serializable
    # Sexual content.
    getter sexual : Bool = false
    # Hate speech.
    getter hate : Bool = false
    # Harassment.
    getter harassment : Bool = false
    # Self-harm content.
    @[JSON::Field(key: "self-harm")]
    getter self_harm : Bool = false
    # Sexual content involving minors.
    @[JSON::Field(key: "sexual/minors")]
    getter sexual_minors : Bool = false
    # Hate speech that threatens violence.
    @[JSON::Field(key: "hate/threatening")]
    getter hate_threatening : Bool = false
    # Graphic violence.
    @[JSON::Field(key: "violence/graphic")]
    getter violence_graphic : Bool = false
    # Intent to self-harm.
    @[JSON::Field(key: "self-harm/intent")]
    getter self_harm_intent : Bool = false
    # Instructions for self-harm.
    @[JSON::Field(key: "self-harm/instructions")]
    getter self_harm_instructions : Bool = false
    # Harassment that threatens violence.
    @[JSON::Field(key: "harassment/threatening")]
    getter harassment_threatening : Bool = false
    # Non-graphic violence.
    getter violence : Bool = false
  end

  # Confidence scores for each moderation category.
  class ModerationCategoryScores
    include JSON::Serializable
    # Sexual content score.
    getter sexual : Float64 = 0.0
    # Hate speech score.
    getter hate : Float64 = 0.0
    # Harassment score.
    getter harassment : Float64 = 0.0
    # Self-harm content score.
    @[JSON::Field(key: "self-harm")]
    getter self_harm : Float64 = 0.0
    # Sexual content involving minors score.
    @[JSON::Field(key: "sexual/minors")]
    getter sexual_minors : Float64 = 0.0
    # Hate speech that threatens violence score.
    @[JSON::Field(key: "hate/threatening")]
    getter hate_threatening : Float64 = 0.0
    # Graphic violence score.
    @[JSON::Field(key: "violence/graphic")]
    getter violence_graphic : Float64 = 0.0
    # Intent to self-harm score.
    @[JSON::Field(key: "self-harm/intent")]
    getter self_harm_intent : Float64 = 0.0
    # Instructions for self-harm score.
    @[JSON::Field(key: "self-harm/instructions")]
    getter self_harm_instructions : Float64 = 0.0
    # Harassment that threatens violence score.
    @[JSON::Field(key: "harassment/threatening")]
    getter harassment_threatening : Float64 = 0.0
    # Non-graphic violence score.
    getter violence : Float64 = 0.0
  end

  # Request to rerank documents by relevance to a query.
  class RerankRequest
    include JSON::Serializable
    # Model ID (e.g., `"cohere/rerank-english-v3.0"`).
    getter model : String = ""
    # The search query.
    getter query : String = ""
    # Documents to rerank.
    getter documents : Array(RerankDocument) = [] of RerankDocument
    # Return only the top N results. Optional.
    getter top_n : UInt32?
    # Include the document content in results. Defaults to false.
    getter return_documents : Bool?
  end

  # Response from the rerank endpoint.
  class RerankResponse
    include JSON::Serializable
    # Unique identifier for this rerank request.
    getter id : String?
    # Reranked documents in order of relevance.
    getter results : Array(RerankResult) = [] of RerankResult
    # Optional metadata about the reranking operation.
    getter meta : JSON::Any?
  end

  # A single reranked document with its relevance score.
  class RerankResult
    include JSON::Serializable
    # Original document index in the input list.
    getter index : UInt32 = 0
    # Relevance score in `[0, 1]`. Higher indicates more relevant.
    getter relevance_score : Float64 = 0.0
    # Original document content (if `return_documents` was true).
    getter document : RerankResultDocument?
  end

  # The text content of a reranked document, returned when `return_documents` is true.
  class RerankResultDocument
    include JSON::Serializable
    # Document text.
    getter text : String = ""
  end

  # A search request.
  class SearchRequest
    include JSON::Serializable
    # The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`).
    getter model : String = ""
    # The search query string.
    getter query : String = ""
    # Maximum number of results to return.
    getter max_results : UInt32?
    # Domain filter — restrict results to specific domains.
    getter search_domain_filter : Array(String)?
    # Country code for localized results (ISO 3166-1 alpha-2, e.g., `"US"`, `"FR"`).
    getter country : String?
  end

  # A search response.
  class SearchResponse
    include JSON::Serializable
    # List of search results.
    getter results : Array(SearchResult) = [] of SearchResult
    # Model/provider that performed the search.
    getter model : String = ""
  end

  # An individual search result.
  class SearchResult
    include JSON::Serializable
    # Result title.
    getter title : String = ""
    # Result URL.
    getter url : String = ""
    # Text snippet or excerpt from the page.
    getter snippet : String = ""
    # Publication or last-updated date, if available.
    getter date : String?
  end

  # An OCR request.
  class OcrRequest
    include JSON::Serializable
    # The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`).
    getter model : String = ""
    # The document to process (URL or base64).
    getter document : OcrDocument
    # Specific pages to process (1-indexed). `None` means all pages.
    getter pages : Array(UInt32)?
    # Whether to include base64-encoded images of each processed page.
    getter include_image_base64 : Bool?
  end

  # An OCR response.
  class OcrResponse
    include JSON::Serializable
    # Extracted pages in order.
    getter pages : Array(OcrPage) = [] of OcrPage
    # Model/provider used for OCR.
    getter model : String = ""
    # Token usage, if reported by the provider.
    getter usage : Usage?
  end

  # A single page of OCR output.
  class OcrPage
    include JSON::Serializable
    # Page index (0-based).
    getter index : UInt32 = 0
    # Extracted page content as Markdown.
    getter markdown : String = ""
    # Embedded images extracted from the page (if `include_image_base64` was true).
    getter images : Array(OcrImage)?
    # Page dimensions in pixels, if available.
    getter dimensions : PageDimensions?
  end

  # An image extracted from an OCR page.
  class OcrImage
    include JSON::Serializable
    # Unique image identifier within the document.
    getter id : String = ""
    # Base64-encoded image data (if `include_image_base64` was true).
    getter image_base64 : String?
  end

  # Page dimensions in pixels.
  class PageDimensions
    include JSON::Serializable
    # Width in pixels.
    getter width : UInt32 = 0
    # Height in pixels.
    getter height : UInt32 = 0
  end

  # Response listing available models.
  class ModelsListResponse
    include JSON::Serializable
    # Always `"list"` from OpenAI-compatible APIs.  Stored as a plain
    # `String` so non-standard provider values do not break deserialization.
    getter object : String = ""
    # List of available models.
    getter data : Array(ModelObject) = [] of ModelObject
  end

  # A model available from the API.
  class ModelObject
    include JSON::Serializable
    # Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`).
    getter id : String = ""
    # Always `"model"` from OpenAI-compatible APIs.  Stored as a plain
    # `String` so non-standard provider values do not break deserialization.
    getter object : String = ""
    # Unix timestamp of model creation (or release date).
    getter created : UInt64 = 0
    # Organization or entity that owns the model.
    getter owned_by : String = ""
  end

  # Request to upload a file.
  class CreateFileRequest
    include JSON::Serializable
    # Base64-encoded file data.
    getter file : String = ""
    # Purpose for the file.
    getter purpose : FilePurpose = FilePurpose::Assistants
    # Optional filename to associate with the upload.
    getter filename : String?
  end

  # An uploaded file object.
  class FileObject
    include JSON::Serializable
    # Unique file ID.
    getter id : String = ""
    # Object type (always `"file"`).
    getter object : String = ""
    # File size in bytes.
    getter bytes : UInt64 = 0
    # Unix timestamp of file creation.
    getter created_at : UInt64 = 0
    # Filename.
    getter filename : String = ""
    # File purpose.
    getter purpose : String = ""
    # Processing status (e.g., `"uploaded"`, `"processed"`).
    getter status : String?
  end

  # Response from listing files.
  class FileListResponse
    include JSON::Serializable
    # Object type (always `"list"`).
    getter object : String = ""
    # List of file objects.
    getter data : Array(FileObject) = [] of FileObject
    # Whether more results are available.
    getter has_more : Bool?
  end

  # Query parameters for listing files.
  class FileListQuery
    include JSON::Serializable
    # Filter by file purpose (e.g., `"batch"`, `"fine-tune"`).
    getter purpose : String?
    # Maximum number of results to return. Defaults to 20.
    getter limit : UInt32?
    # Pagination cursor: return results after this file ID.
    getter after : String?
  end

  # Response from a delete operation.
  class DeleteResponse
    include JSON::Serializable
    # ID of the deleted resource.
    getter id : String = ""
    # Object type.
    getter object : String = ""
    # Confirmation that the resource was deleted.
    getter deleted : Bool = false
  end

  # Request to create a batch job.
  class CreateBatchRequest
    include JSON::Serializable
    # ID of the uploaded input file (JSONL format).
    getter input_file_id : String = ""
    # API endpoint (e.g., `"/v1/chat/completions"`).
    getter endpoint : String = ""
    # Completion window (e.g., `"24h"`).
    getter completion_window : String = ""
    # Optional metadata to attach to the batch.
    getter metadata : JSON::Any?
  end

  # A batch job object.
  class BatchObject
    include JSON::Serializable
    # Unique batch ID.
    getter id : String = ""
    # Object type (always `"batch"`).
    getter object : String = ""
    # API endpoint (e.g., `"/v1/chat/completions"`).
    getter endpoint : String = ""
    # ID of the input file.
    getter input_file_id : String = ""
    # Completion window (e.g., `"24h"`).
    getter completion_window : String = ""
    # Current job status.
    getter status : BatchStatus = BatchStatus::Validating
    # ID of the output file (present when completed).
    getter output_file_id : String?
    # ID of the error file (present if some requests failed).
    getter error_file_id : String?
    # Unix timestamp of batch creation.
    getter created_at : UInt64 = 0
    # Unix timestamp of completion (if completed).
    getter completed_at : UInt64?
    # Unix timestamp of failure (if failed).
    getter failed_at : UInt64?
    # Unix timestamp of expiration (if expired).
    getter expired_at : UInt64?
    # Request processing counts.
    getter request_counts : BatchRequestCounts?
    # Metadata attached to the batch.
    getter metadata : JSON::Any?
  end

  # Request processing counts for a batch.
  class BatchRequestCounts
    include JSON::Serializable
    # Total requests in the batch.
    getter total : UInt64 = 0
    # Completed requests.
    getter completed : UInt64 = 0
    # Failed requests.
    getter failed : UInt64 = 0
  end

  # Response from listing batches.
  class BatchListResponse
    include JSON::Serializable
    # Object type (always `"list"`).
    getter object : String = ""
    # List of batch objects.
    getter data : Array(BatchObject) = [] of BatchObject
    # Whether more results are available.
    getter has_more : Bool?
    # First batch ID in the result set (for pagination).
    getter first_id : String?
    # Last batch ID in the result set (for pagination).
    getter last_id : String?
  end

  # Query parameters for listing batches.
  class BatchListQuery
    include JSON::Serializable
    # Maximum number of results to return. Defaults to 20.
    getter limit : UInt32?
    # Pagination cursor: return results after this batch ID.
    getter after : String?
  end

  # Request to create a structured response.
  class CreateResponseRequest
    include JSON::Serializable
    # Model ID.
    getter model : String = ""
    # Input data to process (e.g., a document to extract from).
    getter input : JSON::Any = JSON::Any.new(nil)
    # Instructions for processing the input.
    getter instructions : String?
    # Available tools the model can use.
    getter tools : Array(ResponseTool)?
    # Sampling temperature in `[0.0, 2.0]`. Defaults to 1.0.
    getter temperature : Float64?
    # Maximum output tokens.
    getter max_output_tokens : UInt64?
    # Optional metadata.
    getter metadata : JSON::Any?
  end

  # A tool available for the response request.
  class ResponseTool
    include JSON::Serializable
    # Tool type (e.g., "extractor", "search").
    @[JSON::Field(key: "type")]
    getter tool_type : String = ""
    # Tool configuration (flattened into the object).
    getter config : JSON::Any = JSON::Any.new(nil)
  end

  # Response from a structured response request.
  class ResponseObject
    include JSON::Serializable
    # Unique response ID.
    getter id : String = ""
    # Object type (e.g., `"response"`).
    getter object : String = ""
    # Unix timestamp of response creation.
    getter created_at : UInt64 = 0
    # Model used to generate the response.
    getter model : String = ""
    # Status (e.g., `"succeeded"`, `"failed"`).
    getter status : String = ""
    # Output items from the response.
    getter output : Array(ResponseOutputItem) = [] of ResponseOutputItem
    # Token usage.
    getter usage : ResponseUsage?
    # Error details (if status is "failed").
    getter error : JSON::Any?
  end

  # A single output item from the response.
  class ResponseOutputItem
    include JSON::Serializable
    # Output type (e.g., `"text"`, `"object"`, `"error"`).
    @[JSON::Field(key: "type")]
    getter item_type : String = ""
    # Output content (flattened into the object).
    getter content : JSON::Any = JSON::Any.new(nil)
  end

  # Token usage for a response.
  class ResponseUsage
    include JSON::Serializable
    # Input tokens used.
    getter input_tokens : UInt64 = 0
    # Output tokens used.
    getter output_tokens : UInt64 = 0
    # Total tokens used.
    getter total_tokens : UInt64 = 0
  end

  # Configuration for polling a batch until terminal status.
  #
  # All time values are in seconds as `f64` so the struct bridges across FFI
  # boundaries without requiring a `Duration` shim.
  class WaitForBatchConfig
    include JSON::Serializable
    # Initial interval between polls, in seconds.
    getter initial_interval_secs : Float64 = 5.0
    # Maximum interval between polls (backoff plateau), in seconds.
    getter max_interval_secs : Float64 = 60.0
    # Exponential backoff multiplier (e.g., 1.5 increases delay by 50% each poll).
    getter backoff_multiplier : Float32 = 1.5
    # Optional timeout in seconds — polling fails if this duration is exceeded.
    getter timeout_secs : Float64?
  end

  # Default client implementation backed by `reqwest`.
  #
  # Sends requests to 143 LLM providers with automatic provider detection
  # and per-request routing. The provider is resolved at construction time
  # from `model_hint` (or defaults to OpenAI), but individual requests can
  # override the provider via model name prefix (e.g. `"anthropic/claude-3-5-sonnet"`
  # routes to Anthropic regardless of construction-time setting).
  #
  # When the model prefix does not match any known provider, the construction-time
  # provider is used as the fallback. This enables seamless migration between
  # providers by changing only the model name.
  #
  # The provider is stored behind an [`Arc`] so it can be shared cheaply into
  # async closures and streaming tasks. Pre-computed auth headers and extra
  # headers are cached at construction to avoid redundant encoding on every request.
  class DefaultClient
    # Wraps the owned FFI handle; do not construct directly.
    def initialize(@handle : Void*)
    end
    # Raw handle for passing back across the C ABI.
    def to_unsafe : Void*
      @handle
    end
    def finalize
      LibLiterllm.default_client_free(@handle) unless @handle.null?
    end
    def fetch_batch_for_polling(batch_id : String) : BatchObject
    __ptr = LibLiterllm.default_client_fetch_batch_for_polling(@handle, batch_id)
    raise "LibLiterllm.default_client_fetch_batch_for_polling returned a null pointer" if __ptr.null?
    __json_ptr = LibLiterllm.batch_object_to_json(__ptr)
    LibLiterllm.batch_object_free(__ptr)
    __json = String.new(__json_ptr)
    LibLiterllm.free_string(__json_ptr)
    BatchObject.from_json(__json)
    end
    # Poll a batch until it reaches a terminal status (Completed, Failed, Expired, Cancelled).
    #
    # Uses exponential backoff with configurable initial interval, maximum interval, and backoff multiplier.
    # Optionally supports a timeout that aborts polling if exceeded.
    # Raises:
    #   Returns `BatchWaitError::Failed` if the batch reaches a failure terminal status.
    # Returns `BatchWaitError::Timeout` if the configured timeout is exceeded.
    # Returns `BatchWaitError::Client` for underlying client errors.
    def wait_for_batch(batch_id : String, config : WaitForBatchConfig) : BatchObject
    __handle_config = LibLiterllm.wait_for_batch_config_from_json(config.to_json)
    __result = begin
          __ptr = LibLiterllm.default_client_wait_for_batch(@handle, batch_id, __handle_config)
          raise "LibLiterllm.default_client_wait_for_batch returned a null pointer" if __ptr.null?
          __json_ptr = LibLiterllm.batch_object_to_json(__ptr)
          LibLiterllm.batch_object_free(__ptr)
          __json = String.new(__json_ptr)
          LibLiterllm.free_string(__json_ptr)
          BatchObject.from_json(__json)
    end
    LibLiterllm.wait_for_batch_config_free(__handle_config)
    __result
    end

    # Stream of `ChatCompletionChunk` items over a fiber-fed channel.
    def chat_stream(req : ChatCompletionRequest) : Channel(ChatCompletionChunk)
    __handle_req = LibLiterllm.chat_completion_request_from_json(req.to_json)
      __handle = LibLiterllm.default_client_chat_stream_start(@handle, __handle_req)
      __ch = Channel(ChatCompletionChunk).new
      raise "LibLiterllm.default_client_chat_stream_start returned a null iterator" if __handle.null?
      spawn do
        begin
          loop do
            __chunk = LibLiterllm.default_client_chat_stream_next(__handle)
            break if __chunk.null?
            __chunk_ptr = __chunk.as(LibLiterllm::ChatCompletionChunk*)
            __jp = LibLiterllm.chat_completion_chunk_to_json(__chunk_ptr)
            if __jp.null?
              LibLiterllm.chat_completion_chunk_free(__chunk_ptr)
              break
            end
            __json = String.new(__jp)
            LibLiterllm.free_string(__jp)
            LibLiterllm.chat_completion_chunk_free(__chunk_ptr)
            __ch.send(ChatCompletionChunk.from_json(__json))
          end
        ensure
          LibLiterllm.default_client_chat_stream_free(__handle)
    LibLiterllm.chat_completion_request_free(__handle_req)
          __ch.close
        end
      end
      __ch
    end

    # Call `chat` via the FFI C ABI.
    def chat(req : ChatCompletionRequest) : ChatCompletionResponse
    __ptr = LibLiterllm.default_client_chat_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_chat_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    ChatCompletionResponse.from_json(__json)
    end

    # Call `embed` via the FFI C ABI.
    def embed(req : EmbeddingRequest) : EmbeddingResponse
    __ptr = LibLiterllm.default_client_embed_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_embed_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    EmbeddingResponse.from_json(__json)
    end

    # Call `list_models` via the FFI C ABI.
    def list_models() : ModelsListResponse
    __ptr = LibLiterllm.default_client_list_models_json(@handle)
    raise "LibLiterllm.default_client_list_models_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    ModelsListResponse.from_json(__json)
    end

    # Call `image_generate` via the FFI C ABI.
    def image_generate(req : CreateImageRequest) : ImagesResponse
    __ptr = LibLiterllm.default_client_image_generate_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_image_generate_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    ImagesResponse.from_json(__json)
    end

    # Call `transcribe` via the FFI C ABI.
    def transcribe(req : CreateTranscriptionRequest) : TranscriptionResponse
    __ptr = LibLiterllm.default_client_transcribe_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_transcribe_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    TranscriptionResponse.from_json(__json)
    end

    # Call `moderate` via the FFI C ABI.
    def moderate(req : ModerationRequest) : ModerationResponse
    __ptr = LibLiterllm.default_client_moderate_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_moderate_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    ModerationResponse.from_json(__json)
    end

    # Call `rerank` via the FFI C ABI.
    def rerank(req : RerankRequest) : RerankResponse
    __ptr = LibLiterllm.default_client_rerank_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_rerank_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    RerankResponse.from_json(__json)
    end

    # Call `search` via the FFI C ABI.
    def search(req : SearchRequest) : SearchResponse
    __ptr = LibLiterllm.default_client_search_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_search_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    SearchResponse.from_json(__json)
    end

    # Call `speech` via the FFI C ABI.
    def speech(req : CreateSpeechRequest) : BytesBytes
    __ptr = LibLiterllm.default_client_speech_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_speech_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    BytesBytes.from_json(__json)
    end

    # Call `ocr` via the FFI C ABI.
    def ocr(req : OcrRequest) : OcrResponse
    __ptr = LibLiterllm.default_client_ocr_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_ocr_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    OcrResponse.from_json(__json)
    end

    # Call `create_file` via the FFI C ABI.
    def create_file(req : CreateFileRequest) : FileObject
    __ptr = LibLiterllm.default_client_create_file_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_create_file_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    FileObject.from_json(__json)
    end

    # Call `retrieve_file` via the FFI C ABI.
    def retrieve_file(file_id : String) : FileObject
    __ptr = LibLiterllm.default_client_retrieve_file_json(@handle, file_id)
    raise "LibLiterllm.default_client_retrieve_file_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    FileObject.from_json(__json)
    end

    # Call `delete_file` via the FFI C ABI.
    def delete_file(file_id : String) : DeleteResponse
    __ptr = LibLiterllm.default_client_delete_file_json(@handle, file_id)
    raise "LibLiterllm.default_client_delete_file_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    DeleteResponse.from_json(__json)
    end

    # Call `list_files` via the FFI C ABI.
    def list_files(query : FileListQuery?) : FileListResponse
    __ptr = LibLiterllm.default_client_list_files_json(@handle, query.to_json)
    raise "LibLiterllm.default_client_list_files_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    FileListResponse.from_json(__json)
    end

    # Call `file_content` via the FFI C ABI.
    def file_content(file_id : String) : BytesBytes
    __ptr = LibLiterllm.default_client_file_content_json(@handle, file_id)
    raise "LibLiterllm.default_client_file_content_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    BytesBytes.from_json(__json)
    end

    # Call `create_batch` via the FFI C ABI.
    def create_batch(req : CreateBatchRequest) : BatchObject
    __ptr = LibLiterllm.default_client_create_batch_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_create_batch_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    BatchObject.from_json(__json)
    end

    # Call `retrieve_batch` via the FFI C ABI.
    def retrieve_batch(batch_id : String) : BatchObject
    __ptr = LibLiterllm.default_client_retrieve_batch_json(@handle, batch_id)
    raise "LibLiterllm.default_client_retrieve_batch_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    BatchObject.from_json(__json)
    end

    # Call `list_batches` via the FFI C ABI.
    def list_batches(query : BatchListQuery?) : BatchListResponse
    __ptr = LibLiterllm.default_client_list_batches_json(@handle, query.to_json)
    raise "LibLiterllm.default_client_list_batches_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    BatchListResponse.from_json(__json)
    end

    # Call `cancel_batch` via the FFI C ABI.
    def cancel_batch(batch_id : String) : BatchObject
    __ptr = LibLiterllm.default_client_cancel_batch_json(@handle, batch_id)
    raise "LibLiterllm.default_client_cancel_batch_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    BatchObject.from_json(__json)
    end

    # Call `create_response` via the FFI C ABI.
    def create_response(req : CreateResponseRequest) : ResponseObject
    __ptr = LibLiterllm.default_client_create_response_json(@handle, req.to_json)
    raise "LibLiterllm.default_client_create_response_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    ResponseObject.from_json(__json)
    end

    # Call `retrieve_response` via the FFI C ABI.
    def retrieve_response(response_id : String) : ResponseObject
    __ptr = LibLiterllm.default_client_retrieve_response_json(@handle, response_id)
    raise "LibLiterllm.default_client_retrieve_response_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    ResponseObject.from_json(__json)
    end

    # Call `cancel_response` via the FFI C ABI.
    def cancel_response(response_id : String) : ResponseObject
    __ptr = LibLiterllm.default_client_cancel_response_json(@handle, response_id)
    raise "LibLiterllm.default_client_cancel_response_json returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    ResponseObject.from_json(__json)
    end
  end

  # Configuration for registering a custom LLM provider at runtime.
  class CustomProviderConfig
    include JSON::Serializable
    # Unique name for this provider (e.g., "my-provider").
    getter name : String = ""
    # Base URL for the provider's API (e.g., `<https://api.my-provider.com/v1>`).
    getter base_url : String = ""
    # Authentication header format.
    getter auth_header : AuthHeaderFormat = AuthHeaderFormat.from_json("{}")
    # Model name prefixes that route to this provider (e.g., `["my-"]`).
    getter model_prefixes : Array(String) = [] of String
  end

  # Static capability flags for a provider.
  #
  # Each flag indicates whether the provider's models *generally* support that
  # feature.  For providers that aggregate many underlying models (e.g. Bedrock,
  # OpenRouter, vLLM) the flags reflect the superset of available model
  # capabilities — a flag being `true` means at least one model supports the
  # feature, not every model.
  #
  # All flags default to `false` so that newly added providers are safe.
  #
  # Access via the crate-level [`capabilities`] function:
  #
  # ```rust
  # use liter_llm::capabilities;
  #
  # let caps = capabilities("openai");
  # assert!(caps.function_calling);
  # assert!(caps.vision);
  #
  # // Unknown providers return a default-all-false reference.
  # let unknown = capabilities("my-private-model");
  # assert!(!unknown.function_calling);
  # ```
  class ProviderCapabilities
    include JSON::Serializable
    # The provider accepts image input in chat messages.
    getter vision : Bool = false
    # The provider supports extended-thinking / reasoning tokens.
    getter reasoning : Bool = false
    # The provider supports JSON-mode or `response_format` structured output.
    getter structured_output : Bool = false
    # The provider supports tool / function calling.
    getter function_calling : Bool = false
    # The provider accepts audio as input.
    getter audio_in : Bool = false
    # The provider can generate audio / TTS output.
    getter audio_out : Bool = false
    # The provider accepts video as input.
    getter video_in : Bool = false
  end

  # Static configuration for a single provider entry in providers.json.
  #
  # This struct deliberately does not include capability flags or streaming
  # format, which are accessed via the [`capabilities`] function.
  class ProviderConfig
    include JSON::Serializable
    # Provider identifier (matches the entry key in providers.json).
    getter name : String = ""
    # Human-readable provider name shown in UIs.
    getter display_name : String?
    # Base URL used as the default for this provider's HTTP client.
    getter base_url : String?
    # Authentication scheme metadata (auth type + env var holding the key).
    getter auth : AuthConfig?
    # Supported endpoint kinds (e.g. `chat`, `embeddings`).
    getter endpoints : Array(String)?
    # Model-name prefixes claimed by this provider (e.g. `["gpt-", "o1-"]`).
    getter model_prefixes : Array(String)?
    # Parameter key renaming for this provider.
    #
    # Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`)
    # to the name this provider expects (e.g. `"max_tokens"`).  Applied
    # automatically by `ConfigDrivenProvider::transform_request`.
    getter param_mappings : Hash(String, String)?
  end

  # Auth configuration block.
  class AuthConfig
    include JSON::Serializable
    # Auth scheme classification.
    @[JSON::Field(key: "type")]
    getter auth_type : AuthType = AuthType::Bearer
    # Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`).
    # Holds the variable name, never the secret value.
    getter env_var : String?
  end

  # Configuration for budget enforcement.
  class BudgetConfig
    include JSON::Serializable
    # Maximum total spend across all models, in USD.  `None` means unlimited.
    getter global_limit : Float64?
    # Per-model spending limits in USD.  Models not listed here are only
    # constrained by `global_limit`.
    getter model_limits : Hash(String, Float64) = {} of String => Float64
    # Whether to reject requests or merely warn when a limit is exceeded.
    getter enforcement : Enforcement = Enforcement::Hard
  end

  # Configuration for the response cache.
  class CacheConfig
    include JSON::Serializable
    # Maximum number of cached entries.
    getter max_entries : UInt64 = 256
    # Time-to-live for each cached entry.
    getter ttl : Int64 = 300000
    # Storage backend to use.
    getter backend : CacheBackend
  end

  # The value broadcast from a singleflight leader to all followers.
  #
  # The error value is shared so every follower receives the same upstream
  # failure without cloning the underlying error.
  class SingleflightResult
    # Wraps the owned FFI handle; do not construct directly.
    def initialize(@handle : Void*)
    end
    # Raw handle for passing back across the C ABI.
    def to_unsafe : Void*
      @handle
    end
    def finalize
      LibLiterllm.singleflight_result_free(@handle) unless @handle.null?
    end
  end

  # Configuration for per-model rate limits.
  class RateLimitConfig
    include JSON::Serializable
    # Maximum requests per window.  `None` means unlimited.
    getter rpm : UInt32?
    # Maximum tokens per window.  `None` means unlimited.
    getter tpm : UInt64?
    # Fixed window duration (defaults to 60 s).
    getter window : Int64 = 60000
  end

  # An intent prototype: `(intent_name, prototype_embedding, target_model_id)`.
  class IntentPrototype
    include JSON::Serializable
    # Human-readable name for the intent (used in logs/metrics).
    getter name : String = ""
    # Pre-computed embedding vector for this intent.
    getter embedding : Array(Float64) = [] of Float64
    # Model to route to when this intent is detected.
    getter model : String = ""
  end

  # A chat message in a conversation.
  abstract class Message
    include JSON::Serializable
    use_json_discriminator "role", {"system" => Message::System, "user" => Message::User, "assistant" => Message::Assistant, "tool" => Message::Tool, "developer" => Message::Developer, "function" => Message::Function}
  end

  class Message::System < Message
    include JSON::Serializable
    @[JSON::Field(key: "role")]
    getter role : String = "system"
    getter content : UserContent
    getter name : String?
  end

  class Message::User < Message
    include JSON::Serializable
    @[JSON::Field(key: "role")]
    getter role : String = "user"
    getter content : UserContent
    getter name : String?
  end

  class Message::Assistant < Message
    include JSON::Serializable
    @[JSON::Field(key: "role")]
    getter role : String = "assistant"
    getter content : AssistantContent?
    getter name : String?
    getter tool_calls : Array(ToolCall)?
    getter refusal : String?
    getter function_call : FunctionCall?
  end

  class Message::Tool < Message
    include JSON::Serializable
    @[JSON::Field(key: "role")]
    getter role : String = "tool"
    getter content : String
    getter tool_call_id : String
    getter name : String?
  end

  class Message::Developer < Message
    include JSON::Serializable
    @[JSON::Field(key: "role")]
    getter role : String = "developer"
    getter content : String
    getter name : String?
  end

  class Message::Function < Message
    include JSON::Serializable
    @[JSON::Field(key: "role")]
    getter role : String = "function"
    getter content : String
    getter name : String
  end

  # User message content as either plain text or a list of multimodal parts.
  abstract class UserContent
    def self.new(pull : ::JSON::PullParser) : UserContent
      __raw = ::JSON::Any.new(pull).to_json
      begin
        return UserContent::Text.from_json(__raw)
      rescue ::JSON::ParseException
      end
      begin
        return UserContent::Parts.from_json(__raw)
      rescue ::JSON::ParseException
      end
      raise ::JSON::ParseException.new("no UserContent variant matched", 0, 0)
    end

    def self.from_json(string : String) : UserContent
      new(::JSON::PullParser.new(string))
    end
    abstract def to_json(json : ::JSON::Builder)
  end

  class UserContent::Text < UserContent
    getter value : String
    def initialize(@value : String)
    end
    def self.from_json(string : String) : UserContent::Text
      new(String.from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  class UserContent::Parts < UserContent
    getter value : Array(ContentPart)
    def initialize(@value : Array(ContentPart))
    end
    def self.from_json(string : String) : UserContent::Parts
      new(Array(ContentPart).from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  # A single content part in a user message — text, image, document, or audio.
  abstract class ContentPart
    include JSON::Serializable
    use_json_discriminator "type", {"text" => ContentPart::Text, "image_url" => ContentPart::ImageUrl, "document" => ContentPart::Document, "input_audio" => ContentPart::InputAudio}
  end

  class ContentPart::Text < ContentPart
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "text"
    getter text : String
  end

  class ContentPart::ImageUrl < ContentPart
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "image_url"
    getter image_url : ImageUrl
  end

  class ContentPart::Document < ContentPart
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "document"
    getter document : DocumentContent
  end

  class ContentPart::InputAudio < ContentPart
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "input_audio"
    getter input_audio : AudioContent
  end

  # Image detail level controlling token cost and processing.
  enum ImageDetail
    Low
    High
    Auto
  end

  # Content shape for assistant messages.
  #
  # `#[serde(untagged)]` means providers returning a plain scalar string for the
  # `content` field still deserialise correctly into `AssistantContent::Text(_)`.
  # Providers returning an array of typed parts (e.g. after an image-generation
  # or audio-synthesis request) deserialise into `AssistantContent::Parts(_)`.
  abstract class AssistantContent
    def self.new(pull : ::JSON::PullParser) : AssistantContent
      __raw = ::JSON::Any.new(pull).to_json
      begin
        return AssistantContent::Text.from_json(__raw)
      rescue ::JSON::ParseException
      end
      begin
        return AssistantContent::Parts.from_json(__raw)
      rescue ::JSON::ParseException
      end
      raise ::JSON::ParseException.new("no AssistantContent variant matched", 0, 0)
    end

    def self.from_json(string : String) : AssistantContent
      new(::JSON::PullParser.new(string))
    end
    abstract def to_json(json : ::JSON::Builder)
  end

  class AssistantContent::Text < AssistantContent
    getter value : String
    def initialize(@value : String)
    end
    def self.from_json(string : String) : AssistantContent::Text
      new(String.from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  class AssistantContent::Parts < AssistantContent
    getter value : Array(AssistantPart)
    def initialize(@value : Array(AssistantPart))
    end
    def self.from_json(string : String) : AssistantContent::Parts
      new(Array(AssistantPart).from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  # One part of a structured assistant response.
  #
  # `#[serde(tag = "type", rename_all = "snake_case")]` matches OpenAI's
  # parts-spec discriminator (`"type": "text"`, `"type": "output_image"`, …).
  abstract class AssistantPart
    include JSON::Serializable
    use_json_discriminator "type", {"text" => AssistantPart::Text, "refusal" => AssistantPart::Refusal, "output_image" => AssistantPart::OutputImage, "output_audio" => AssistantPart::OutputAudio}
  end

  class AssistantPart::Text < AssistantPart
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "text"
    getter text : String
  end

  class AssistantPart::Refusal < AssistantPart
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "refusal"
    getter refusal : String
  end

  class AssistantPart::OutputImage < AssistantPart
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "output_image"
    getter image_url : ImageUrl
  end

  class AssistantPart::OutputAudio < AssistantPart
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "output_audio"
    getter audio : AudioContent
  end

  # The type discriminator for tool/tool-call objects.
  #
  # Per the OpenAI spec this is always `"function"`. Using an enum enforces
  # that constraint at the type level and rejects any other value on
  # deserialization.
  enum ToolType
    Function
  end

  # Tool usage mode or a specific tool to call.
  abstract class ToolChoice
    def self.new(pull : ::JSON::PullParser) : ToolChoice
      __raw = ::JSON::Any.new(pull).to_json
      begin
        return ToolChoice::Mode.from_json(__raw)
      rescue ::JSON::ParseException
      end
      begin
        return ToolChoice::Specific.from_json(__raw)
      rescue ::JSON::ParseException
      end
      raise ::JSON::ParseException.new("no ToolChoice variant matched", 0, 0)
    end

    def self.from_json(string : String) : ToolChoice
      new(::JSON::PullParser.new(string))
    end
    abstract def to_json(json : ::JSON::Builder)
  end

  class ToolChoice::Mode < ToolChoice
    getter value : ToolChoiceMode
    def initialize(@value : ToolChoiceMode)
    end
    def self.from_json(string : String) : ToolChoice::Mode
      new(ToolChoiceMode.from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  class ToolChoice::Specific < ToolChoice
    getter value : SpecificToolChoice
    def initialize(@value : SpecificToolChoice)
    end
    def self.from_json(string : String) : ToolChoice::Specific
      new(SpecificToolChoice.from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  # Tool choice mode.
  enum ToolChoiceMode
    Auto
    Required
    None
  end

  # Wire format for the chat completions `response_format` field.
  #
  # # Provider mapping
  #
  # - **OpenAI** (and OpenAI-compatible providers): emitted verbatim as
  #   `{"type": "json_schema", "json_schema": {...}}` per the
  #   chat-completions spec.
  # - **Gemini / Vertex AI**: translated to
  #   `generationConfig.responseMimeType = "application/json"` and
  #   `generationConfig.responseSchema = <schema>`. The `name`,
  #   `description`, and `strict` fields are dropped — Gemini's
  #   structured-output API does not consume them.
  # - **Anthropic**: no native JSON mode. A system instruction is
  #   prepended asking the model to respond with valid JSON.
  #   `strict` is advisory only; callers should still validate the
  #   returned JSON if the schema is load-bearing.
  abstract class ResponseFormat
    include JSON::Serializable
    use_json_discriminator "type", {"text" => ResponseFormat::Text, "json_object" => ResponseFormat::JsonObject, "json_schema" => ResponseFormat::JsonSchema}
  end

  class ResponseFormat::Text < ResponseFormat
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "text"
  end

  class ResponseFormat::JsonObject < ResponseFormat
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "json_object"
  end

  class ResponseFormat::JsonSchema < ResponseFormat
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "json_schema"
    getter json_schema : JsonSchemaFormat
  end

  # Stop sequence(s) that cause the model to stop generating.
  abstract class StopSequence
    def self.new(pull : ::JSON::PullParser) : StopSequence
      __raw = ::JSON::Any.new(pull).to_json
      begin
        return StopSequence::Single.from_json(__raw)
      rescue ::JSON::ParseException
      end
      begin
        return StopSequence::Multiple.from_json(__raw)
      rescue ::JSON::ParseException
      end
      raise ::JSON::ParseException.new("no StopSequence variant matched", 0, 0)
    end

    def self.from_json(string : String) : StopSequence
      new(::JSON::PullParser.new(string))
    end
    abstract def to_json(json : ::JSON::Builder)
  end

  class StopSequence::Single < StopSequence
    getter value : String
    def initialize(@value : String)
    end
    def self.from_json(string : String) : StopSequence::Single
      new(String.from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  class StopSequence::Multiple < StopSequence
    getter value : Array(String)
    def initialize(@value : Array(String))
    end
    def self.from_json(string : String) : StopSequence::Multiple
      new(Array(String).from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  # Output modality requested from the model.
  #
  # Passed as `modalities: ["text", "audio"]` (OpenAI) or translated to
  # `generationConfig.responseModalities` (Gemini / Vertex AI).
  enum Modality
    Text
    Audio
    Image
  end

  # Why a choice stopped generating tokens.
  enum FinishReason
    Stop
    Length
    ToolCalls
    ContentFilter
    FunctionCall
    Other
  end

  # Controls how much reasoning effort the model should use.
  enum ReasoningEffort
    Low
    Medium
    High
  end

  # The format in which the embedding vectors are returned.
  enum EmbeddingFormat
    Float
    Base64
  end

  # Text or texts to embed.
  abstract class EmbeddingInput
    def self.new(pull : ::JSON::PullParser) : EmbeddingInput
      __raw = ::JSON::Any.new(pull).to_json
      begin
        return EmbeddingInput::Single.from_json(__raw)
      rescue ::JSON::ParseException
      end
      begin
        return EmbeddingInput::Multiple.from_json(__raw)
      rescue ::JSON::ParseException
      end
      raise ::JSON::ParseException.new("no EmbeddingInput variant matched", 0, 0)
    end

    def self.from_json(string : String) : EmbeddingInput
      new(::JSON::PullParser.new(string))
    end
    abstract def to_json(json : ::JSON::Builder)
  end

  class EmbeddingInput::Single < EmbeddingInput
    getter value : String
    def initialize(@value : String)
    end
    def self.from_json(string : String) : EmbeddingInput::Single
      new(String.from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  class EmbeddingInput::Multiple < EmbeddingInput
    getter value : Array(String)
    def initialize(@value : Array(String))
    end
    def self.from_json(string : String) : EmbeddingInput::Multiple
      new(Array(String).from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  # Input to the moderation endpoint — a single string or multiple strings.
  abstract class ModerationInput
    def self.new(pull : ::JSON::PullParser) : ModerationInput
      __raw = ::JSON::Any.new(pull).to_json
      begin
        return ModerationInput::Single.from_json(__raw)
      rescue ::JSON::ParseException
      end
      begin
        return ModerationInput::Multiple.from_json(__raw)
      rescue ::JSON::ParseException
      end
      raise ::JSON::ParseException.new("no ModerationInput variant matched", 0, 0)
    end

    def self.from_json(string : String) : ModerationInput
      new(::JSON::PullParser.new(string))
    end
    abstract def to_json(json : ::JSON::Builder)
  end

  class ModerationInput::Single < ModerationInput
    getter value : String
    def initialize(@value : String)
    end
    def self.from_json(string : String) : ModerationInput::Single
      new(String.from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  class ModerationInput::Multiple < ModerationInput
    getter value : Array(String)
    def initialize(@value : Array(String))
    end
    def self.from_json(string : String) : ModerationInput::Multiple
      new(Array(String).from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  # A document to be reranked — either a plain string or an object with a text field.
  abstract class RerankDocument
    def self.new(pull : ::JSON::PullParser) : RerankDocument
      __raw = ::JSON::Any.new(pull).to_json
      begin
        return RerankDocument::Text.from_json(__raw)
      rescue ::JSON::ParseException
      end
      begin
        return RerankDocument::Object.from_json(__raw)
      rescue ::JSON::ParseException
      end
      raise ::JSON::ParseException.new("no RerankDocument variant matched", 0, 0)
    end

    def self.from_json(string : String) : RerankDocument
      new(::JSON::PullParser.new(string))
    end
    abstract def to_json(json : ::JSON::Builder)
  end

  class RerankDocument::Text < RerankDocument
    getter value : String
    def initialize(@value : String)
    end
    def self.from_json(string : String) : RerankDocument::Text
      new(String.from_json(string))
    end
    def to_json(json : ::JSON::Builder)
      @value.to_json(json)
    end
  end

  class RerankDocument::Object < RerankDocument
    include JSON::Serializable
    getter text : String
  end

  # Document input for OCR — either a URL or inline base64 data.
  abstract class OcrDocument
    include JSON::Serializable
    use_json_discriminator "type", {"document_url" => OcrDocument::Url, "base64" => OcrDocument::Base64}
  end

  class OcrDocument::Url < OcrDocument
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "document_url"
    getter url : String
  end

  class OcrDocument::Base64 < OcrDocument
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "base64"
    getter data : String
    getter media_type : String
  end

  # Purpose of an uploaded file.
  enum FilePurpose
    Assistants
    Batch
    FineTune
    Vision
  end

  # Status of a batch job.
  enum BatchStatus
    Validating
    Failed
    InProgress
    Finalizing
    Completed
    Expired
    Cancelling
    Cancelled
  end

  # How the API key is sent in the HTTP request.
  abstract class AuthHeaderFormat
    def self.new(pull : ::JSON::PullParser) : AuthHeaderFormat
      case pull.kind
      when .string?
        __tag = pull.read_string
        case __tag
        when "Bearer" then return AuthHeaderFormat::Bearer.new
        when "None" then return AuthHeaderFormat::None.new
        else raise ::JSON::ParseException.new("unknown AuthHeaderFormat variant: #{__tag}", *pull.location)
        end
      when .begin_object?
        __result : AuthHeaderFormat? = nil
        pull.read_object do |__key|
          case __key
          when "ApiKey" then __result = AuthHeaderFormat::ApiKey.new(pull)
          else pull.skip
          end
        end
        return __result || raise ::JSON::ParseException.new("empty AuthHeaderFormat object", *pull.location)
      else
        raise ::JSON::ParseException.new("invalid AuthHeaderFormat JSON", *pull.location)
      end
    end

    def self.from_json(string : String) : AuthHeaderFormat
      new(::JSON::PullParser.new(string))
    end

    abstract def to_json(json : ::JSON::Builder)
  end

  class AuthHeaderFormat::Bearer < AuthHeaderFormat
    def to_json(json : ::JSON::Builder)
      json.string("Bearer")
    end
  end

  class AuthHeaderFormat::ApiKey < AuthHeaderFormat
    getter value : String
    def initialize(@value : String)
    end
    def self.new(pull : ::JSON::PullParser) : AuthHeaderFormat::ApiKey
      __v = String.new(pull)
      new(__v)
    end
    def to_json(json : ::JSON::Builder)
      json.object do
        json.field("ApiKey") do
          @value.to_json(json)
        end
      end
    end
  end

  class AuthHeaderFormat::None < AuthHeaderFormat
    def to_json(json : ::JSON::Builder)
      json.string("None")
    end
  end

  # The streaming wire format a provider uses for its response stream.
  #
  # Most providers use standard Server-Sent Events (SSE).  AWS Bedrock uses
  # a proprietary binary EventStream framing.
  #
  # Deserialized from the `streaming_format` JSON field via [`serde`].
  enum StreamFormat
    Sse
    AwsEventStream
  end

  # Auth scheme used by a provider.
  enum AuthType
    Bearer
    ApiKey
    None
    Unknown
  end

  # How budget limits are enforced.
  enum Enforcement
    Hard
    Soft
  end

  # Storage backend for the response cache.
  abstract class CacheBackend
    include JSON::Serializable
    use_json_discriminator "type", {"memory" => CacheBackend::Memory, "open_dal" => CacheBackend::OpenDal}
  end

  class CacheBackend::Memory < CacheBackend
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "memory"
  end

  class CacheBackend::OpenDal < CacheBackend
    include JSON::Serializable
    @[JSON::Field(key: "type")]
    getter type_ : String = "open_dal"
    getter scheme : String
    getter config : Hash(String, String)
  end

  # Observable state of a circuit breaker.
  enum CircuitState
    Closed
    Open
    HalfOpen
  end

  # The result of a single health probe.
  enum HealthStatus
    Healthy
    Unhealthy
  end

  # All errors that can occur when using `liter-llm`.
  class LiterLlmError < Exception
  end

  # Create a new LLM client with simple scalar configuration.
  def self.create_client(api_key : String, base_url : String?, timeout_secs : UInt64?, max_retries : UInt32?, model_hint : String?) : DefaultClient
    __ptr = LibLiterllm.create_client(api_key, base_url, timeout_secs, max_retries, model_hint)
    raise "LibLiterllm.create_client returned a null pointer" if __ptr.null?
    DefaultClient.new(__ptr)
  end

  # Create a new LLM client from a JSON string.
  def self.create_client_from_json(json : String) : DefaultClient
    __ptr = LibLiterllm.create_client_from_json(json)
    raise "LibLiterllm.create_client_from_json returned a null pointer" if __ptr.null?
    DefaultClient.new(__ptr)
  end

  # Encode bytes as a base64 data URL: `data:<mime>;base64,<b64>`.
  def self.encode_data_url(bytes : Bytes, mime : String?) : String
    __ptr = LibLiterllm.encode_data_url(bytes.to_a.to_json, mime)
    raise "LibLiterllm.encode_data_url returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    __json
  end

  # Decode a base64 data URL into [`DecodedDataUrl`].
  def self.decode_data_url(url : String) : DecodedDataUrl?
    __ptr = LibLiterllm.decode_data_url(url)
    return nil if __ptr.null?
    __json_ptr = LibLiterllm.decoded_data_url_to_json(__ptr)
    LibLiterllm.decoded_data_url_free(__ptr)
    __json = String.new(__json_ptr)
    LibLiterllm.free_string(__json_ptr)
    DecodedDataUrl.from_json(__json)
  end

  # Register a custom provider in the global runtime registry.
  def self.register_custom_provider(config : CustomProviderConfig) : Nil
    __handle_config = LibLiterllm.custom_provider_config_from_json(config.to_json)
    __result = LibLiterllm.register_custom_provider(__handle_config)
    __code = LibLiterllm.last_error_code
    if __code != 0
      __ctx_ptr = LibLiterllm.last_error_context
      raise String.new(__ctx_ptr) unless __ctx_ptr.null?
      raise "unknown error"
    end
    __result
    LibLiterllm.custom_provider_config_free(__handle_config)
  end

  # Remove a previously registered custom provider by name.
  def self.unregister_custom_provider(name : String) : Bool
    __result = LibLiterllm.unregister_custom_provider(name)
    __code = LibLiterllm.last_error_code
    if __code != 0
      __ctx_ptr = LibLiterllm.last_error_context
      raise String.new(__ctx_ptr) unless __ctx_ptr.null?
      raise "unknown error"
    end
    __result
  end

  # Return the capability flags for a named provider.
  def self.capabilities(provider_name : String) : ProviderCapabilities
    __ptr = LibLiterllm.capabilities(provider_name)
    raise "LibLiterllm.capabilities returned a null pointer" if __ptr.null?
    __json_ptr = LibLiterllm.provider_capabilities_to_json(__ptr)
    LibLiterllm.provider_capabilities_free(__ptr)
    __json = String.new(__json_ptr)
    LibLiterllm.free_string(__json_ptr)
    ProviderCapabilities.from_json(__json)
  end

  # Return all provider configs from the registry.
  def self.all_providers() : Array(ProviderConfig)
    __ptr = LibLiterllm.all_providers()
    raise "LibLiterllm.all_providers returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    Array(ProviderConfig).from_json(__json)
  end

  # Return the set of complex provider names.
  def self.complex_provider_names() : Array(String)
    __ptr = LibLiterllm.complex_provider_names()
    raise "LibLiterllm.complex_provider_names returned a null pointer" if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    Array(String).from_json(__json)
  end

  # Calculate the estimated cost of a completion given a model name and token
  def self.completion_cost(model : String, prompt_tokens : UInt64, completion_tokens : UInt64) : Float64?
    __ptr = LibLiterllm.completion_cost(model, prompt_tokens, completion_tokens)
    return nil if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    Float64.from_json(__json)
  end

  # Calculate the estimated cost of a completion, accounting for cached
  def self.completion_cost_with_cache(model : String, prompt_tokens : UInt64, cached_tokens : UInt64, completion_tokens : UInt64) : Float64?
    __ptr = LibLiterllm.completion_cost_with_cache(model, prompt_tokens, cached_tokens, completion_tokens)
    return nil if __ptr.null?
    __json = String.new(__ptr)
    LibLiterllm.free_string(__ptr)
    Float64.from_json(__json)
  end

  # Remove all guardrails from the global registry.
  def self.clear() : Nil
    LibLiterllm.clear()
    nil
  end

  # Count tokens in a text string using the tokenizer for the given model.
  def self.count_tokens(model : String, text : String) : UInt64
    __result = LibLiterllm.count_tokens(model, text)
    __code = LibLiterllm.last_error_code
    if __code != 0
      __ctx_ptr = LibLiterllm.last_error_context
      raise String.new(__ctx_ptr) unless __ctx_ptr.null?
      raise "unknown error"
    end
    __result
  end

  # Count tokens for a full [`ChatCompletionRequest`].
  def self.count_request_tokens(model : String, req : ChatCompletionRequest) : UInt64
    __handle_req = LibLiterllm.chat_completion_request_from_json(req.to_json)
    __result = LibLiterllm.count_request_tokens(model, __handle_req)
    __code = LibLiterllm.last_error_code
    if __code != 0
      __ctx_ptr = LibLiterllm.last_error_context
      raise String.new(__ctx_ptr) unless __ctx_ptr.null?
      raise "unknown error"
    end
    LibLiterllm.chat_completion_request_free(__handle_req)
    __result
  end

  # Assert that `current_len + incoming` does not exceed `limit`.
  def self.check_bound(context : String, current_len : UInt64, incoming : UInt64, limit : UInt64) : Nil
    __result = LibLiterllm.check_bound(context, current_len, incoming, limit)
    __code = LibLiterllm.last_error_code
    if __code != 0
      __ctx_ptr = LibLiterllm.last_error_context
      raise String.new(__ctx_ptr) unless __ctx_ptr.null?
      raise "unknown error"
    end
    __result
  end

  # Install the `ring` crypto provider as the rustls process default, idempotently.
  def self.ensure_crypto_provider() : Nil
    LibLiterllm.ensure_crypto_provider()
    nil
  end
end
