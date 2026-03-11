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
