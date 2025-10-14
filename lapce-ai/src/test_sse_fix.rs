// Test to understand SSE parser issue
fn main() {
    println!("Test SSE parsing issue:");
    
    let input = b"data: {\"text\":\"Hello\"}\n\ndata: {\"text\":\"World\"}\n\ndata: [DONE]\n\n";
    println!("Input: {:?}", std::str::from_utf8(input).unwrap());
    
    // The parser should find:
    // Line 1: "data: {\"text\":\"Hello\"}"
    // Line 2: "" (empty - triggers event)
    // Line 3: "data: {\"text\":\"World\"}"
    // Line 4: "" (empty - triggers event)
    // Line 5: "data: [DONE]"
    // Line 6: "" (empty - triggers event)
    
    let mut pos = 0;
    let mut line_num = 0;
    for (i, &b) in input.iter().enumerate() {
        if b == b'\n' {
            let line = &input[pos..i];
            println!("Line {}: {:?}", line_num, std::str::from_utf8(line).unwrap_or("<invalid>"));
            pos = i + 1;
            line_num += 1;
        }
    }
}
