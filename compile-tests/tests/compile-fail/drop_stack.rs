extern crate ref_mut_stack;

use ref_mut_stack::{ParkableRefMut, RefMutStack};

fn main() {
    struct Root;

    let mut root = Root;

    struct Type<'a>(ParkableRefMut<'a, Root, Self>);

    let mut stack = RefMutStack::<Root, Type>::new(&mut root);
    let r = stack.borrow_mut();
    drop(stack);
    //~^ ERROR 14:10: 14:15: cannot move out of `stack` because it is borrowed [E0505]
}
