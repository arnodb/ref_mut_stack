use ref_mut_stack::{ParkableRefMut, RefMutStack};

// We want to provide a way to the user to use the references in destructors.
//
// But because of the existence of such a destructor, the reference needs to be detached from
// `self` while unparking. This is why it is wrapped with an `Option`.

struct Builder<'a>(Option<ParkableRefMut<'a, usize, Self>>, usize);

impl<'a> Builder<'a> {
    fn build(mut self) -> Option<Self> {
        let r = self.0.take().unwrap();
        r.unpark()
    }
}

impl<'a> Drop for Builder<'a> {
    fn drop(&mut self) {
        if let Some(r) = self.0.as_mut() {
            **r = self.1
        };
    }
}

#[test]
pub fn test_sound_incomplete_unstacking() {
    let mut root = 12;

    {
        let mut stack = RefMutStack::<usize, Builder>::new(&mut root);
        let mut b1 = Builder(Some(stack.borrow_mut()), 100);
        let mut b2 = Builder(Some(b1.0.as_mut().unwrap().parker().park(b1, |r| r)), 200);
        let mut b3 = Builder(Some(b2.0.as_mut().unwrap().parker().park(b2, |r| r)), 300);
        let b4 = Builder(Some(b3.0.as_mut().unwrap().parker().park(b3, |r| r)), 400);

        let Some(_b3) = b4.build() else {
            panic!("b4 build should return b3")
        };
        // `b3`, `b2` and `b1` are not built.
        //
        // The last 2 are still in the stack and need to be dropped in order for the process to be
        // sound.
        //
        // Everything is dropped here.
    }

    // If not 100 that means the destructors are not called in the right order, which indicates
    // unsound behaviour (detected by Miri).
    assert_eq!(root, 100);
}

#[test]
pub fn test_sound_unwinding() {
    // Need a lock to cross the catch_unwind scope safely
    let root = std::sync::Mutex::new(12);

    std::panic::catch_unwind(|| {
        let mut root = root.lock().unwrap();

        let mut stack = RefMutStack::<usize, Builder>::new(&mut root);
        let mut b1 = Builder(Some(stack.borrow_mut()), 100);
        let mut b2 = Builder(Some(b1.0.as_mut().unwrap().parker().park(b1, |r| r)), 200);
        let mut b3 = Builder(Some(b2.0.as_mut().unwrap().parker().park(b2, |r| r)), 300);
        let b4 = Builder(Some(b3.0.as_mut().unwrap().parker().park(b3, |r| r)), 400);

        // Drop root to make sure the lock is not poisoned
        drop(b4);
        drop(root);

        // 3 builders are still in the stack and need to be dropped in order for the process to be
        // sound.

        panic!("intentional panic");
    })
    .unwrap_err();

    // If not 100 that means the destructors are not called in the right order, which indicates
    // unsound behaviour (detected by Miri).
    assert_eq!(*root.lock().unwrap(), 100);
}
