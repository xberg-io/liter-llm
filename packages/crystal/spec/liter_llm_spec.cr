require "./spec_helper"

describe LiterLlm do
  it "exposes a version" do
    LiterLlm::VERSION.should_not be_empty
  end
end
