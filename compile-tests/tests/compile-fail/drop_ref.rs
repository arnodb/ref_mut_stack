extern crate ref_mut_stack;

use ref_mut_stack::{ParkableRefMut, RefMutStack};

fn main() {
    struct Root;

    let mut root = Root;

    struct Type<'a>(ParkableRefMut<'a, Root, Self>);

    let mut stack = RefMutStack::<Root, Type>::new(&mut root);
    drop(root);
    //~^ ERROR 13:10: 13:14: cannot move out of `root` because it is borrowed [E0505]
    stack.borrow_mut();
}
