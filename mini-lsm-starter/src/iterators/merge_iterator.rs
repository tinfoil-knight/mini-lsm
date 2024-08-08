#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use std::cmp::{self};
use std::collections::binary_heap::PeekMut;
use std::collections::BinaryHeap;

use anyhow::Result;

use crate::key::KeySlice;

use super::StorageIterator;

struct HeapWrapper<I: StorageIterator>(pub usize, pub Box<I>);

impl<I: StorageIterator> PartialEq for HeapWrapper<I> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl<I: StorageIterator> Eq for HeapWrapper<I> {}

impl<I: StorageIterator> PartialOrd for HeapWrapper<I> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: StorageIterator> Ord for HeapWrapper<I> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.1
            .key()
            .cmp(&other.1.key())
            .then(self.0.cmp(&other.0))
            .reverse()
    }
}

/// Merge multiple iterators of the same type. If the same key occurs multiple times in some
/// iterators, prefer the one with smaller index.
pub struct MergeIterator<I: StorageIterator> {
    iters: BinaryHeap<HeapWrapper<I>>,
    current: Option<HeapWrapper<I>>,
}

impl<I: StorageIterator> MergeIterator<I> {
    pub fn create(iters: Vec<Box<I>>) -> Self {
        let mut heap: BinaryHeap<HeapWrapper<I>> = iters
            .into_iter()
            .filter(|x| x.is_valid())
            .enumerate()
            .map(|f| HeapWrapper(f.0, f.1))
            .collect();

        let current = heap.pop();
        Self {
            iters: heap,
            current,
        }
    }
}

impl<I: 'static + for<'a> StorageIterator<KeyType<'a> = KeySlice<'a>>> StorageIterator
    for MergeIterator<I>
{
    type KeyType<'a> = KeySlice<'a>;

    // key(), value(), next() are only supposed to be called if is_valid returns true

    fn key(&self) -> KeySlice {
        self.current.as_ref().unwrap().1.key()
    }

    fn value(&self) -> &[u8] {
        self.current.as_ref().unwrap().1.value()
    }

    fn is_valid(&self) -> bool {
        self.current.as_ref().map_or(false, |hw| hw.1.is_valid())
    }

    fn next(&mut self) -> Result<()> {
        // SEEN 1.2T2
        // Didn't realise that the Ord trait implemented for the HeapWrapper
        // already arranged the heap as per first key of the iter.
        // Also had some issues with assigning a value to self.current using a reference.
        // Solution was to use .as_mut, .as_ref

        let current = self.current.as_mut().unwrap();

        // Skip duplicate key
        while let Some(mut iter) = self.iters.peek_mut() {
            if iter.1.key() == current.1.key() {
                if let e @ Err(_) = iter.1.next() {
                    PeekMut::pop(iter);
                    return e;
                }

                if !iter.1.is_valid() {
                    PeekMut::pop(iter);
                }
            } else {
                break;
            }
        }

        current.1.next()?;

        if !current.1.is_valid() {
            if let Some(iter) = self.iters.pop() {
                *current = iter
            }
            return Ok(());
        }

        if let Some(mut iter) = self.iters.peek_mut() {
            // Ord uses reverse for the max-binary heap
            if *current < *iter {
                std::mem::swap(&mut *iter, current);
            }
        }

        Ok(())
    }
}
