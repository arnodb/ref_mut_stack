# RefMutStack
[![Latest Version](https://img.shields.io/crates/v/ref_mut_stack)](https://crates.io/crates/ref_mut_stack)
[![Documentation](https://docs.rs/ref_mut_stack/badge.svg)](https://docs.rs/ref_mut_stack)
[![Build Status](https://github.com/arnodb/ref_mut_stack/actions/workflows/ci.yml/badge.svg)](https://github.com/arnodb/ref_mut_stack/actions/workflows/ci.yml)
[![Code Coverage](https://codecov.io/gh/arnodb/ref_mut_stack/branch/main/graph/badge.svg)](https://codecov.io/gh/arnodb/ref_mut_stack)

RefMutStack allows to simulate recursion where each level holds a mutable reference to the one held by the caller with an iteration.

It is made in such a way that the rules enforced by the borrow checker during the theoretical recursion are still enforced during iterations. On that purpose, each object holding a mutable reference becomes unreachable when the recursion is similated: it is stacked until it becomes usable again.

## Soundness

RefMutStack should be sound in many ways and some tests are implemented to ensure it remains sound. However there might be unsound cases which would very likely be attempts at abusing it - we don't judge.

The "`impl Drop` using the held mutable reference" case is even protected but note that it would easily qualify as "abuse". Keep it simple and everything will be fine.

## Example

[owned_builder.rs](examples/owned_builder.rs) shows an example of a builder used to iteratively build a tree which should be built recursively otherwise (to leverage the true borrow checker):

```rust
use ref_mut_stack::{ParkableRefMut, RefMutStack};

#[derive(Default, Debug, PartialEq, Eq)]
struct Node {
    value: usize,
    child: Option<Box<Node>>,
    built_value: usize,
}

struct Builder<'a> {
    node: ParkableRefMut<'a, Node, Self>,
}

impl<'a> Builder<'a> {
    fn value(mut self, value: usize) -> Self {
        self.node.value = value;
        self
    }

    fn new_child(mut self) -> Self {
        if self.node.child.is_some() {
            panic!();
        }
        self.node.child = Some(Box::new(Node::default()));
        let child = self
            .node
            .parker()
            .park(self, |node| node.child.as_mut().unwrap());
        Self { node: child }
    }

    fn build(mut self) -> Option<Self> {
        self.node.built_value = self
            .node
            .child
            .as_ref()
            .map_or(self.node.value, |child| child.built_value + 1);
        self.node.unpark()
    }
}

#[test]
fn test_owned_builder() {
    let mut root = Node::default();

    let mut stack = RefMutStack::new(&mut root);
    let b = Builder {
        node: stack.borrow_mut(),
    }
    .value(1)
    .new_child()
    .value(2)
    .new_child()
    .value(3)
    .build()
    .unwrap()
    .build()
    .unwrap()
    .build();
    if b.is_some() {
        panic!()
    }

    assert_eq!(
        root,
        Node {
            value: 1,
            child: Some(Box::new(Node {
                value: 2,
                child: Some(Box::new(Node {
                    value: 3,
                    child: None,
                    built_value: 3
                })),
                built_value: 4
            })),
            built_value: 5
        }
    );
}
```

## Note from the author

I created this because I needed to traverse a tree branch and do something on the leaf node and was pretty annoyed I needed recursion and callbacks to leverage the borrow checker.

RefMutStack enables me to change my implementation to an iteration and a call to manipulate the leaf node.

Yes, it replaces the thread stack by a stack on the heap. Some would think it is bad, some would find it cool, I don't judge.
