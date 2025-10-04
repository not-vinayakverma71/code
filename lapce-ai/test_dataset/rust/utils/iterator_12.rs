pub struct CustomIterator_12 {
    items: Vec<i32>,
    index: usize,
}

impl Iterator for CustomIterator_12 {
    type Item = i32;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.items.len() {
            let item = self.items[self.index];
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub fn fibonacci_iterator_12() -> impl Iterator<Item = u64> {
    let mut a = 0;
    let mut b = 1;
    std::iter::from_fn(move || {
        let c = a + b;
        a = b;
        b = c;
        Some(a)
    })
}