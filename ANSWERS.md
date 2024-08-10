# WEEK 1

## DAY 1
* Why doesn't the memtable provide a `delete` API?
    - Putting an empty byte-array is equivalent to a `delete` operation for the parent Lsm structure. That's why a `delete` API isn't needed.
* Is it possible to use other data structures as the memtable in LSM? What are the pros/cons of using the skiplist?
    - Yes. Any data structure that supports setting and getting key-value is sufficient.
      - It being ordered isn't necessary if a sequential scan operation isn't needed.
    - Difficult to use contiguous memory in Skiplists compared to BTree.
    - Skiplists are probabilistic in nature and might get unbalanced with time.
    - Skiplists are faster than BTree for in-memory workloads.
* Why do we need a combination of `state` and `state_lock`? Can we only use `state.read()` and `state.write()`?
    - We're acquiring a write-lock on `state` to prevent any other r-w operations on the current memtable. We need to acquire the lock on `state_lock` before it to prevent multiple threads from freezing the current memtable since they'd get the lock one by one & the second thread would just freeze an barely-filled memtable.
    - The `put` method checks if the size of current memtable has reached capacity or not but the method force freezing the memtable shouldn't do this since it can be called in another scenarios (eg. graceful shutdown) which aren't triggered by the memtable reaching capacity. It should be able to acquire a write-lock and prevent any other operations though.
* Why does the order to store and to probe the memtables matter? If a key appears in multiple memtables, which version should you return to the user?
    - We'd use the latest version of the key. The order matters since multiple memtables might have the same key but the correct value is the most recent value which is stored in the latest memtable.
* Is the memory layout of the memtable efficient / does it have good data locality? (Think of how `Byte` is implemented and stored in the skiplist...) What are the possible optimizations to make the memtable more efficient?
    - ?
* So we are using `parking_lot` locks in this tutorial. Is its read-write lock a fair lock? What might happen to the readers trying to acquire the lock if there is one writer waiting for existing readers to stop?
    - ?
    - As per its documentation: "readers trying to acquire the lock will block even if the lock is unlocked when there are writers waiting to acquire the lock. Because of this, attempts to recursively acquire a read lock within a single thread may result in a deadlock." Ref: https://docs.rs/parking_lot/latest/parking_lot/type.RwLock.html
* After freezing the memtable, is it possible that some threads still hold the old LSM state and wrote into these immutable memtables? How does your solution prevent it from happening?
    - We acquire a write lock on current memtable and a lock of the current LSM state before changing the current memtable. It isn't possible for other threads to hold the old LSM state and write to the now immutable memtable.
* There are several places that you might first acquire a read lock on state, then drop it and acquire a write lock (these two operations might be in different functions but they happened sequentially due to one function calls the other). How does it differ from directly upgrading the read lock to a write lock? Is it necessary to upgrade instead of acquiring and dropping and what is the cost of doing the upgrade?
    - ?

## DAY 2
* What is the time/space complexity of using your merge iterator?
 - The memtable and the binary heap holding the memtable iterators are already sorted.
 - In the worst case, I'd need to check and skip a duplicate item from the head of every iterator.
 - The worst case time complexity of getting a single element from the iterator would be O(n) where n is the no. of memtables.
* Why do we need a self-referential structure for memtable iterator?
 - So that the iterator doesn't refer to an invalid memtable that is already freed from memory.
* If a key is removed (there is a delete tombstone), do you need to return it to the user? Where did you handle this logic?
 - No. In the implementation for LsmStorageInner which calls the MemTable.put() method.
* If a key has multiple versions, will the user see all of them? Where did you handle this logic?
 - The user will only see the recent-most update for the key. We're iterating over all memtables in order in the LsmStorageInner.get() method.
* If we want to get rid of self-referential structure and have a lifetime on the memtable iterator (i.e., `MemtableIterator<'a>`, where `'a` = memtable or `LsmStorageInner` lifetime), is it still possible to implement the `scan` functionality?
  - ?
* What happens if (1) we create an iterator on the skiplist memtable (2) someone inserts new keys into the memtable (3) will the iterator see the new key?
  - For the crossbeam_skiplist map, if the keys inserted are after the current iterator position then the iterator will see them otherwise not.
* What happens if your key comparator cannot give the binary heap implementation a stable order?
  - If the binary heap doesn't maintain the insertion order when it encounters duplicates, we'd getting stale values for keys because it would re-order the memtables.
* Why do we need to ensure the merge iterator returns data in the iterator construction order?
  - Because the recent most update for a key is in the latest table.
* Is it possible to implement a Rust-style iterator (i.e., `next(&self) -> (Key, Value)`) for LSM iterators? What are the pros/cons?
* The scan interface is like `fn scan(&self, lower: Bound<&[u8]>, upper: Bound<&[u8]>)`. How to make this API compatible with Rust-style range (i.e., `key_a..key_b`)? If you implement this, try to pass a full range `..` to the interface and see what will happen.
* The starter code provides the merge iterator interface to store `Box<I>` instead of `I`. What might be the reason behind that?
