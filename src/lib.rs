use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

pub struct RefMutStack<'a, R, T> {
    root_ref: NonNull<R>,
    stack: SafeDropVec<(T, NonNull<R>)>,
    _a: std::marker::PhantomData<&'a ()>,
}

impl<'a, R, T> RefMutStack<'a, R, T> {
    pub fn new(r: &'a mut R) -> Self {
        Self {
            root_ref: NonNull::from_mut(r),
            stack: Vec::new().into(),
            _a: Default::default(),
        }
    }

    pub fn borrow_mut(&'a mut self) -> ParkableRefMut<'a, R, T> {
        let r = self.stack.last_mut().map_or(self.root_ref, |(_, r)| *r);
        ParkableRefMut {
            r,
            stack: NonNull::from_mut(self),
        }
    }
}

/// This is a Vec which will drop its elements in reverse order on destruction.
///
/// It ensures soundness of error cases when the elements holding the mutable references really
/// want to access them during their destruction. See soundness integration tests.
///
/// Note that is would be an anti-pattern to use the references in destructors, but this wrapper
/// addresses incorrect implementations.
///
/// Also note that the borrow checker will not allow us to implement `Drop` on [`RefMutStack`]
/// itself because of its self referencing nature. The borrow checker does not complain on the
/// implementation but on the use site when it needs to drop the stack and realizes there are
/// conflicting lifetime requirements.
struct SafeDropVec<T>(Vec<T>);

impl<T> From<Vec<T>> for SafeDropVec<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T> Drop for SafeDropVec<T> {
    fn drop(&mut self) {
        // Drop elements in reverse order.
        while self.0.pop().is_some() {}
    }
}

impl<T> Deref for SafeDropVec<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for SafeDropVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct ParkableRefMut<'a, R, T> {
    r: NonNull<R>,
    stack: NonNull<RefMutStack<'a, R, T>>,
}

impl<'a, R, T> ParkableRefMut<'a, R, T> {
    pub fn parker(&mut self) -> Parker<'a, R, T> {
        Parker {
            r: self.r,
            stack: self.stack,
        }
    }

    pub fn unpark(mut self) -> Option<T> {
        let stack = unsafe { self.stack.as_mut() };
        stack.stack.pop().map(|(v, _)| v)
    }
}

impl<'a, R, T> Deref for ParkableRefMut<'a, R, T> {
    type Target = R;
    fn deref(&self) -> &Self::Target {
        unsafe { self.r.as_ref() }
    }
}

impl<'a, R, T> DerefMut for ParkableRefMut<'a, R, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.r.as_mut() }
    }
}

#[must_use]
pub struct Parker<'a, R, T> {
    r: NonNull<R>,
    stack: NonNull<RefMutStack<'a, R, T>>,
}

impl<'a, R, T> Parker<'a, R, T> {
    pub fn park<F>(mut self, holder: T, f: F) -> ParkableRefMut<'a, R, T>
    where
        R: 'a,
        T: 'a,
        F: Fn(&'a mut R) -> &'a mut R,
    {
        let stack = unsafe { self.stack.as_mut() };
        stack
            .stack
            .push((holder, NonNull::from_mut(f(unsafe { self.r.as_mut() }))));
        stack.borrow_mut()
    }
}
