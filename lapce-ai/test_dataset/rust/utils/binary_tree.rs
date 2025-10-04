use std::cmp::Ordering;

pub struct BinaryTree<T: Ord> {
    root: Option<Box<Node<T>>>,
}

struct Node<T: Ord> {
    value: T,
    left: Option<Box<Node<T>>>,
    right: Option<Box<Node<T>>>,
}

impl<T: Ord> BinaryTree<T> {
    pub fn new() -> Self {
        BinaryTree { root: None }
    }
    
    pub fn insert(&mut self, value: T) {
        match self.root {
            None => self.root = Some(Box::new(Node::new(value))),
            Some(ref mut node) => node.insert(value),
        }
    }
    
    pub fn search(&self, value: &T) -> bool {
        match self.root {
            None => false,
            Some(ref node) => node.search(value),
        }
    }
}