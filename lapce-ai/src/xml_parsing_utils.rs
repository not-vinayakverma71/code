
/// XmlMatcherResult interface - exact translation lines 1-4
#[derive(Debug, Clone)]
pub struct XmlMatcherResult {
    pub matched: bool,
    pub data: String,
}

/// XmlMatcher class - exact translation lines 5-104
pub struct XmlMatcher<Result = XmlMatcherResult> 
where
    Result: Clone,
{
    index: usize,
    chunks: Vec<XmlMatcherResult>,
    cached: Vec<String>,
    matched: bool,
    state: XmlState,
    depth: usize,
    pointer: usize,
    tag_name: String,
    transform: Option<Box<dyn Fn(XmlMatcherResult) -> Result + Send + Sync>>,
    position: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum XmlState {
    Text,
    TagOpen,
    TagClose,
}

impl<Result> XmlMatcher<Result>
where
    Result: Clone,
{
    /// constructor - exact translation lines 13-17
    pub fn new(
        tag_name: String,
        transform: Option<Box<dyn Fn(XmlMatcherResult) -> Result + Send + Sync>>,
        position: Option<usize>,
    ) -> Self {
        Self {
            index: 0,
            chunks: Vec::new(),
            cached: Vec::new(),
            matched: false,
            state: XmlState::Text,
            depth: 0,
            pointer: 0,
            tag_name,
            transform,
            position: position.unwrap_or(0),
        }
    }
    
    pub fn push(&mut self, chunk: &str) {
        self.cached.push(chunk.to_string());
    }
    
    pub fn finish(&self) -> Vec<Result> {
        Vec::new()
    }
    
    /// collect - exact translation lines 18-34
    fn collect(&mut self) {
        if self.cached.is_empty() {
            return;
        }
        
        let data = self.cached.join("");
        let matched = self.matched;
        
        if let Some(last) = self.chunks.last_mut() {
            if last.matched == matched {
                last.data.push_str(&data);
            } else {
                self.chunks.push(XmlMatcherResult {
                    data,
                    matched,
                });
            }
        } else {
            self.chunks.push(XmlMatcherResult {
                data,
                matched,
            });
        }
        
        self.cached.clear();
    }
    
    /// pop - exact translation lines 35-42
    fn pop(&mut self) -> Vec<Result> {
        let chunks = std::mem::take(&mut self.chunks);
        
        if let Some(ref transform) = self.transform {
            chunks.into_iter().map(|chunk| transform(chunk)).collect()
        } else {
            // If no transform, cast XmlMatcherResult to Result
            // This requires Result = XmlMatcherResult
            chunks.into_iter().map(|chunk| {
                // Unsafe cast - assumes Result is XmlMatcherResult
                unsafe { std::mem::transmute_copy(&chunk) }
            }).collect()
        }
    }
    
    /// _update - exact translation lines 44-92
    fn _update(&mut self, chunk: &str) {
        for char in chunk.chars() {
            self.cached.push(char.to_string());
            self.pointer += 1;
            
            match self.state {
                XmlState::Text => {
                    if char == '<' && (self.pointer <= self.position + 1 || self.matched) {
                        self.state = XmlState::TagOpen;
                        self.index = 0;
                    } else {
                        self.collect();
                    }
                }
                XmlState::TagOpen => {
                    if char == '>' && self.index == self.tag_name.len() {
                        self.state = XmlState::Text;
                        if !self.matched {
                            self.cached.clear();
                        }
                        self.depth += 1;
                        self.matched = true;
                    } else if self.index == 0 && char == '/' {
                        self.state = XmlState::TagClose;
                    } else if char == ' ' && (self.index == 0 || self.index == self.tag_name.len()) {
                        continue;
                    } else if self.index < self.tag_name.len() && 
                              self.tag_name.chars().nth(self.index) == Some(char) {
                        self.index += 1;
                    } else {
                        self.state = XmlState::Text;
                        self.collect();
                    }
                }
                XmlState::TagClose => {
                    if char == '>' && self.index == self.tag_name.len() {
                        self.state = XmlState::Text;
                        self.depth -= 1;
                        self.matched = self.depth > 0;
                        if !self.matched {
                            self.cached.clear();
                        }
                    } else if char == ' ' && (self.index == 0 || self.index == self.tag_name.len()) {
                        continue;
                    } else if self.index < self.tag_name.len() && 
                              self.tag_name.chars().nth(self.index) == Some(char) {
                        self.index += 1;
                    } else {
                        self.state = XmlState::Text;
                        self.collect();
                    }
                }
            }
        }
    }
    
    /// final - exact translation lines 93-99
    pub fn final_parse(mut self, chunk: Option<&str>) -> Vec<Result> {
        if let Some(chunk) = chunk {
            self._update(chunk);
        }
        self.collect();
        self.pop()
    }
    
    /// update - exact translation lines 100-103
    pub fn update(&mut self, chunk: &str) -> Vec<Result> {
        self._update(chunk);
        self.pop()
    }
}

/// Default implementation for XmlMatcherResult
impl Default for XmlMatcher<XmlMatcherResult> {
    fn default() -> Self {
        Self::new("".to_string(), None, None)
    }
}

/// Simplified XML matcher for common use cases
pub struct SimpleXmlMatcher {
    matcher: XmlMatcher<XmlMatcherResult>,
}

impl SimpleXmlMatcher {
    pub fn new(tag_name: &str) -> Self {
        Self {
            matcher: XmlMatcher::new(tag_name.to_string(), None, None),
        }
    }
    
    pub fn push(&mut self, text: &str) -> Vec<XmlMatcherResult> {
        self.matcher.update(text)
    }
    
    pub fn finish(self) -> Vec<XmlMatcherResult> {
        self.matcher.final_parse(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_xml_matcher_basic() {
        let mut matcher = SimpleXmlMatcher::new("thinking");
        
        let results = matcher.push("<thinking>test content</thinking>");
        assert!(!results.is_empty());
        
        let final_results = matcher.finish();
        assert!(!final_results.is_empty());
    }
    
    #[test]
    fn test_xml_matcher_nested() {
        let mut matcher = SimpleXmlMatcher::new("outer");
        
        matcher.push("<outer>");
        matcher.push("content");
        matcher.push("<outer>nested</outer>");
        matcher.push("</outer>");
        
        let results = matcher.finish();
        assert!(!results.is_empty());
    }
    
    #[test]
    fn test_xml_matcher_state_transitions() {
        let mut matcher = XmlMatcher::<XmlMatcherResult>::new(
            "test".to_string(),
            None,
            None,
        );
        
        // Test state transitions
        matcher.update("<");
        assert_eq!(matcher.state, XmlState::TagOpen);
        
        matcher.update("test>");
        assert_eq!(matcher.state, XmlState::Text);
        assert!(matcher.matched);
        
        matcher.update("</test>");
        assert!(!matcher.matched);
    }
}
