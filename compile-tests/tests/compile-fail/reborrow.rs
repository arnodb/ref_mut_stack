extern crate ref_mut_stack;

use ref_mut_stack::{ParkableRefMut, RefMutStack};

fn main() -> () {
    let mut root = ();

    struct Type<'a>(ParkableRefMut<'a, (), Self>);

    let mut stack = RefMutStack::<(), Type>::new(&mut root);
    let mut b1 = Type(stack.borrow_mut());
    let _b2 = b1.0.parker().park(b1, |r| r);
    let b3 = Type(stack.borrow_mut());
    //~^ ERROR 13:19: 13:24: cannot borrow `stack` as mutable more than once at a time [E0499]
}
