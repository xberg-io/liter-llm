require "./spec_helper"

describe LiterLlm do
  it "links the generated binding" do
    LiterLlm::VERSION.should_not be_empty
  end
end
