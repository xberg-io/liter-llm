<!-- snippet:compile-only -->

```zig
const liter_llm = @import("liter_llm");
const std = @import("std");

pub fn main() !void {
    const base_url = "http://localhost:11434/v1";
    var client = try liter_llm.create_client("", base_url, null, null, null);
    defer client.close();
    const req = "{\"model\":\"ollama/qwen2:0.5b\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello!\"}]}";
    const response = try client.chat(req);
    defer liter_llm._free_string(response);
    std.debug.print("Response: {s}\n", .{response});
}
```
