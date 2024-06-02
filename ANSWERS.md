# WEEK 1

## DAY 1
* Why doesn't the memtable provide a `delete` API?
    - Putting an empty byte-array is equivalent to a `delete` operation for the parent Lsm structure. That's why a `delete` API isn't needed.
* Is it possible to use other data structures as the memtable in LSM? What are the pros/cons of using the skiplist?
    - ?
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