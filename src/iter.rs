use core::cell::Cell;
use core::future::Future;
use core::marker::Unpin;
use core::ops::Deref;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use pin_project::pin_project;

use crate::waker;

struct Item<T> {
    value: T,
    waker: Waker,
}

pub struct Extractor<T> {
    item: Cell<Option<Item<T>>>,
}

impl<T> Extractor<T> {
    pub const fn new() -> Self {
        Self {
            item: Cell::new(None),
        }
    }
}

impl<T> Extractor<T> {
    pub fn output(&self, value: T) -> impl Future<Output = ()> + '_ {
        YieldFuture {
            target: self,
            queued: Some(value),
        }
    }

    pub fn take(&self) -> Option<T> {
        match self.item.take() {
            Some(Item { value, waker }) => {
                waker.wake();
                Some(value)
            }
            None => None,
        }
    }
}

struct YieldFuture<'a, T> {
    target: &'a Extractor<T>,
    queued: Option<T>,
}

impl<T> Future for YieldFuture<'_, T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(mut previous) = self.target.item.take() {
            previous.waker = cx.waker().clone();
            self.target.item.set(Some(previous));
            return Poll::Pending;
        }

        let me = Pin::into_inner(self);
        match me.queued.take() {
            Some(value) => {
                me.target.item.set(Some(Item {
                    value,
                    waker: cx.waker().clone(),
                }));
                Poll::Pending
            }
            None => Poll::Ready(()),
        }
    }
}

// Why we can implement Unpin here:
//
// * Unpin is safe trait and we don't have any Unsafe around this future or related things.
// * We are not doing anything like a self-referential struct or such. We implement the future the
//   "old" manual way as if there was no async-await around and these didn't need any Pin and
//   everything was Unpin back then.
impl<T> Unpin for YieldFuture<'_, T> {}

#[pin_project]
pub struct YieldIterator<E, Fut> {
    extractor: E,
    #[pin]
    generator: Option<Fut>,
}

impl<E, Fut, T> YieldIterator<E, Fut>
where
    E: Deref<Target = Extractor<T>>,
    Fut: Future<Output = ()>,
{
    pub fn new(extractor: E, f: Fut) -> Self {
        Self {
            extractor,
            generator: Some(f),
        }
    }
    fn next_inner(self: Pin<&mut Self>) -> Option<T> {
        let mut me = self.project();

        loop {
            if let Some(value) = me.extractor.take() {
                break Some(value);
            }

            match me.generator.as_mut().as_pin_mut() {
                Some(fut) => {
                    let waker = waker::null();
                    let mut context = Context::from_waker(&waker);
                    match fut.poll(&mut context) {
                        Poll::Ready(()) => {
                            // We are done with the future, it won't give us more items. It might
                            // have created one right now (though it's not likely), so we loop once
                            // more to check.
                            me.generator.set(None);
                        }
                        // Nothing to do but it might have produced another item -> loop around
                        // once more.
                        Poll::Pending => (),
                    }
                }
                None => break None,
            }
        }
    }
}

impl<E, Fut, T> Iterator for Pin<&mut YieldIterator<E, Fut>>
where
    E: Deref<Target = Extractor<T>>,
    Fut: Future<Output = ()>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        Pin::as_mut(self).next_inner()
    }
}

#[macro_export]
macro_rules! gen_iter {
    (let $name: ident = |$ex: ident| $block: block) => {
        let $ex = $crate::iter::Extractor::new();
        let mut $name = $crate::iter::YieldIterator::new(&$ex, async { $block });
        #[allow(unused_mut)]
        let mut $name = unsafe { core::pin::Pin::new_unchecked(&mut $name) };
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_use() {
        let extractor = Extractor::new();
        let mut it = YieldIterator::new(&extractor, async {
            extractor.output(42).await;
            extractor.output(12).await;
        });
        let it = unsafe { Pin::new_unchecked(&mut it) };

        assert_eq!(it.collect::<Vec<_>>(), vec![42, 12]);
    }

    #[test]
    fn macro_use() {
        gen_iter!(let it = |extractor| {
            extractor.output(42).await;
            extractor.output(12).await;
        });

        assert_eq!(it.collect::<Vec<_>>(), vec![42, 12]);
    }
}
