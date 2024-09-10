# WEEK 1

## DAY 1
* Why doesn't the memtable provide a `delete` API?
    - Putting an empty byte-array is equivalent to a `delete` operation for the parent Lsm structure. That's why a `delete` API isn't needed.
* Is it possible to use other data structures as the memtable in LSM? What are the pros/cons of using the skiplist?
    - Yes. Any data structure that supports setting and getting key-value is sufficient.
      - It being ordered isn't necessary since a sequential scan operation isn't needed.
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

## DAY 3
* What is the time complexity of seeking a key in the block?
- If you don't have the offset, then the time complexity would be O(logn) since a block stores ordered keys and we can perform binary search.
* Where does the cursor stop when you seek a non-existent key in your implementation?
- At the end since I'm seeking sequentially instead of performing a binary search.
* So `Block` is simply a vector of raw data and a vector of offsets. Can we change them to `Byte` and `Arc<[u16]>`, and change all the iterator interfaces to return `Byte` instead of `&[u8]`? (Assume that we use `Byte::slice` to return a slice of the block without copying.) What are the pros/cons?
* What is the endian of the numbers written into the blocks in your implementation?
- Little endian
* Is your implementation prune to a maliciously-built block? Will there be invalid memory access, or OOMs, if a user deliberately construct an invalid block?
- Yes. Yes.
* Can a block contain duplicated keys?
- Yes.
* What happens if the user adds a key larger than the target block size?
- If it's not the first key being added to the block, it's ignored.
* Consider the case that the LSM engine is built on object store services (S3). How would you optimize/change the block format and parameters to make it suitable for such services?
- ?

## DAY 4
* What is the time complexity of seeking a key in the SST?
- Finding the block index should be O(logm) with binary search. Combined with then finding the key in the ordered block, the time complexity would be O(logm) + O(logn).
- where m -> no. of blocks ; n -> no. of keys in the block
* Where does the cursor stop when you seek a non-existent key in your implementation?
* Is it possible (or necessary) to do in-place updates of SST files?
- In-place updates can be used to compact an SST file. It it possible to do so.
* An SST is usually large (i.e., 256MB). In this case, the cost of copying/expanding the `Vec` would be significant. Does your implementation allocate enough space for your SST builder in advance? How did you implement it?
* Looking at the `moka` block cache, why does it return `Arc<Error>` instead of the original `Error`?
* Does the usage of a block cache guarantee that there will be at most a fixed number of blocks in memory? For example, if you have a `moka` block cache of 4GB and block size of 4KB, will there be more than 4GB/4KB number of blocks in memory at the same time?
- The moka-rs library should evict additional items over the fixed limit.
* Is it possible to store columnar data (i.e., a table of 100 integer columns) in an LSM engine? Is the current SST format still a good choice?
- Its possible to store columnar data in an LSM engine.
- For storage, data can be split into groups. For eg. col1 for 1-100, col2 for 1-100, ...
* Consider the case that the LSM engine is built on object store services (i.e., S3). How would you optimize/change the SST format/parameters and the block cache to make it suitable for such services?
* For now, we load the index of all SSTs into the memory. Assume you have a 16GB memory reserved for the indexes, can you estimate the maximum size of the database your LSM system can support? (That's why you need an index cache!)
1,2,3,4,5,.....,col1,size
## DAY 5
* Consider the case that a user has an iterator that iterates the whole storage engine, and the storage engine is 1TB large, so that it takes ~1 hour to scan all the data.
  What would be the problems if the user does so? (This is a good question and we will ask it several times at different points of the tutorial...)
- User might get stale data if there are updates of exisiting keys after the iteration has started.
- Might interfere with compaction.
* Another popular interface provided by some LSM-tree storage engines is multi-get (or vectored get). The user can pass a list of keys that they want to retrieve.
  The interface returns the value of each of the key. For example, `multi_get(vec!["a", "b", "c", "d"]) -> a=1,b=2,c=3,d=4`. Obviously, an easy implementation is to simply doing a single get for each of the key.
  How will you implement the multi-get interface, and what optimizations you can do to make it more efficient? (Hint: some operations during the get process will only need to be done once for all keys, and besides that, you can think of an improved disk I/O interface to better support this multi-get interface).
- Keep a map of keys that haven't resolved to a value or "deleted" yet and check each iterator item you come across for the key.
