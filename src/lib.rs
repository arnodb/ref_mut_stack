#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

/// The stack of mutable references and their holders.
///
/// It allows to simulate recursion where each level holds a mutable reference to the one held by
/// the caller with an iteration.
///
/// It is made in such a way that the rules enforced by the borrow checker during the theoretical
/// recursion are still enforced during iterations. On that purpose, each object holding a mutable
/// reference becomes unreachable when the recursion is similated: it is stacked until it becomes
/// usable again.
///
/// In order to use it:
///
/// * create a new stack with the root mutable reference using [`RefMutStack::new`]
/// * call [`RefMutStack::borrow_mut`] to create the root holder of type `T`
/// * enable recursion in `T` itself by calling [`ParkableRefMut::parker`] and park the holder with
///   [`Parker::park`]
/// * finish recursion by calling [`ParkableRefMut::unpark`]
///
/// See builder examples in `tests` for more details.
pub struct RefMutStack<'a, R, T> {
    root_ref: NonNull<R>,
    stack: SafeDropVec<(T, NonNull<R>)>,
    _a: std::marker::PhantomData<&'a ()>,
}

impl<'a, R, T> RefMutStack<'a, R, T> {
    /// Creates a new stack with the root mutable reference.
    pub fn new(r: &'a mut R) -> Self {
        Self {
            root_ref: NonNull::from_mut(r),
            stack: Vec::new().into(),
            _a: Default::default(),
        }
    }

    /// Borrows the current mutable reference at the top of the stack in order to use it.
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
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for SafeDropVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Holder of one mutable reference.
///
/// It can park itself and its holder of type `T` via [`ParkableRefMut::parker`] and [`Parker::park`].
///
/// It can unpark itself via [`ParkableRefMut::unpark`] which returns the previously parked holder of type `T` if
/// any, `None` if everything has been unparked.
pub struct ParkableRefMut<'a, R, T> {
    r: NonNull<R>,
    stack: NonNull<RefMutStack<'a, R, T>>,
}

impl<'a, R, T> ParkableRefMut<'a, R, T> {
    /// Creates a new parker to park the mutable reference holder and derive a new one.
    pub fn parker(&mut self) -> Parker<'a, R, T> {
        Parker {
            r: self.r,
            stack: self.stack,
        }
    }

    /// Unparks the reference and returns the previously parked holder of type `T` if any, `None`
    /// if everything has been unparked.
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

/// The parker helper.
///
/// It must be used by calling [`Parker::park`], passing the reference holder of type `T` which
/// becomes unreachable until it is unparked.
#[must_use]
pub struct Parker<'a, R, T> {
    r: NonNull<R>,
    stack: NonNull<RefMutStack<'a, R, T>>,
}

impl<'a, R, T> Parker<'a, R, T> {
    /// Parks the reference holder which becomes unreachable until it is unparked.
    ///
    /// The callback is used to derive a new mutable reference from the current one in the stack.
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
