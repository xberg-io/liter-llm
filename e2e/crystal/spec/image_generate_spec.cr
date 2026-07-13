require "./spec_helper"

describe LiterLlm do
  describe "image-generate" do
    pending "Image generation returning base64-encoded data instead of URL"
    pending "Image generation with an empty prompt returns 400"
    pending "Image generation requesting multiple images with n=3"
    pending "401 Unauthorized when generating images with invalid API key"
    pending "400 Bad Request when image generation parameters are invalid"
    pending "429 Rate limit exceeded for image generation"
    pending "Basic image generation with a text prompt"
    pending "Image generation requesting multiple images"
    pending "Image generation with explicit size parameter"
  end
end
