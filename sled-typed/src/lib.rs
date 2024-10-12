//! This is based on the typed_sled crate with some fundamental changes
//! - serialization errors do not cause panics
//! - various features I didn't need removed
//! - big endian number encoding so that ordering in sled works properly
//! - tuple based prefix iteration restores the functionality of scan_prefix
//! - use anyhow for errors, because I like anyhow, and it's
//!   neccessary to unify sled errors with serialization errors to
//!   maintain an ergonomic interface

pub mod transaction;

use anyhow::Result;
use bincode::Options;
use serde::Serialize;
pub use sled::{open, Config};
use sled::{
    transaction::{ConflictableTransactionResult, TransactionResult},
    IVec,
};
use std::{
    fmt,
    iter::{DoubleEndedIterator, Iterator},
    marker::PhantomData,
    ops::{Bound, RangeBounds},
};
use transaction::TransactionalTree;

/// A flash-sympathetic persistent lock-free B+ tree.
///
/// A `Tree` represents a single logical keyspace / namespace / bucket.
///
/// # Example
/// ```
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// struct SomeValue(u32);
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Creating a temporary sled database.
///     // If you want to persist the data use sled::open instead.
///     let db = sled::Config::new().temporary(true).open().unwrap();
///
///     // The id is used by sled to identify which Tree in the database (db) to open.
///     let tree = typed_sled::Tree::<String, SomeValue>::open(&db, "unique_id")?;
///
///     tree.insert(&"some_key".to_owned(), &SomeValue(10))?;
///
///     assert_eq!(tree.get(&"some_key".to_owned())?, Some(SomeValue(10)));
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct Tree<K, V> {
    inner: sled::Tree,
    _key: PhantomData<K>,
    _value: PhantomData<V>,
}

/// Trait alias for bounds required on keys and values.
/// For now only types that implement DeserializeOwned
/// are supported.
pub trait KV: serde::de::DeserializeOwned + Serialize {}

impl<T: serde::de::DeserializeOwned + Serialize> KV for T {}

/// marker trait for types that serialize to a valid prefix of another
/// type for the purposes of sled. Used by subscribe, scan_prefix,
/// etc.
///
/// You can implement this yourself for structs and can rely on
/// serde visiting struct fields in the order they are lexicially
/// defined.
pub trait Prefix<T>: KV {}

impl<T: KV> Prefix<T> for T {}

impl<A: KV, B: KV> Prefix<A> for (A, B) {}
impl<A: KV, B: KV, C: KV> Prefix<A> for (A, B, C) {}
impl<A: KV, B: KV, C: KV> Prefix<(A, B)> for (A, B, C) {}
impl<A: KV, B: KV, C: KV, D: KV> Prefix<A> for (A, B, C, D) {}
impl<A: KV, B: KV, C: KV, D: KV> Prefix<(A, B)> for (A, B, C, D) {}
impl<A: KV, B: KV, C: KV, D: KV> Prefix<(A, B, C)> for (A, B, C, D) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV> Prefix<A> for (A, B, C, D, E) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV> Prefix<(A, B)> for (A, B, C, D, E) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV> Prefix<(A, B, C)> for (A, B, C, D, E) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV> Prefix<(A, B, C, D)> for (A, B, C, D, E) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV> Prefix<A> for (A, B, C, D, E, F) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV> Prefix<(A, B)> for (A, B, C, D, E, F) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV> Prefix<(A, B, C)> for (A, B, C, D, E, F) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV> Prefix<(A, B, C, D)> for (A, B, C, D, E, F) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV> Prefix<(A, B, C, D, E)> for (A, B, C, D, E, F) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV> Prefix<A> for (A, B, C, D, E, F, G) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV> Prefix<(A, B)> for (A, B, C, D, E, F, G) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV> Prefix<(A, B, C)> for (A, B, C, D, E, F, G) {}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV> Prefix<(A, B, C, D)>
    for (A, B, C, D, E, F, G)
{
}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV> Prefix<(A, B, C, D, E)>
    for (A, B, C, D, E, F, G)
{
}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV> Prefix<(A, B, C, D, E, F)>
    for (A, B, C, D, E, F, G)
{
}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV, H: KV> Prefix<A>
    for (A, B, C, D, E, F, G, H)
{
}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV, H: KV> Prefix<(A, B)>
    for (A, B, C, D, E, F, G, H)
{
}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV, H: KV> Prefix<(A, B, C)>
    for (A, B, C, D, E, F, G, H)
{
}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV, H: KV> Prefix<(A, B, C, D)>
    for (A, B, C, D, E, F, G, H)
{
}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV, H: KV> Prefix<(A, B, C, D, E)>
    for (A, B, C, D, E, F, G, H)
{
}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV, H: KV> Prefix<(A, B, C, D, E, F)>
    for (A, B, C, D, E, F, G, H)
{
}
impl<A: KV, B: KV, C: KV, D: KV, E: KV, F: KV, G: KV, H: KV> Prefix<(A, B, C, D, E, F, G)>
    for (A, B, C, D, E, F, G, H)
{
}

/// Compare and swap error.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompareAndSwapError<V> {
    /// The current value which caused your CAS to fail.
    pub current: Option<V>,
    /// Returned value that was proposed unsuccessfully.
    pub proposed: Option<V>,
}

impl<V> fmt::Display for CompareAndSwapError<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Compare and swap conflict")
    }
}

// implemented like this in the sled source
impl<V: std::fmt::Debug> std::error::Error for CompareAndSwapError<V> {}

// These Trait bounds should probably be specified on the functions themselves, but too lazy.
impl<K, V> Tree<K, V> {
    /// Initialize a typed tree. The id identifies the tree to be opened from the db.
    /// # Example
    ///
    /// ```
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    /// struct SomeValue(u32);
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     // Creating a temporary sled database.
    ///     // If you want to persist the data use sled::open instead.
    ///     let db = sled::Config::new().temporary(true).open().unwrap();
    ///
    ///     // The id is used by sled to identify which Tree in the database (db) to open.
    ///     let tree = typed_sled::Tree::<String, SomeValue>::open(&db, "unique_id");
    ///
    ///     tree.insert(&"some_key".to_owned(), &SomeValue(10))?;
    ///
    ///     assert_eq!(tree.get(&"some_key".to_owned())?, Some(SomeValue(10)));
    ///     Ok(())
    /// }
    /// ```
    pub fn open<T: AsRef<str>>(db: &sled::Db, id: T) -> Self {
        Self {
            inner: db.open_tree(id.as_ref()).unwrap(),
            _key: PhantomData,
            _value: PhantomData,
        }
    }

    /// Insert a key to a new value, returning the last value if it was set.
    pub fn insert(&self, key: &K, value: &V) -> Result<Option<V>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .insert(serialize(key)?, serialize(value)?)?
            .map(|old| deserialize(&old))
            .transpose()?)
    }

    /// Perform a multi-key serializable transaction.
    pub fn transaction<F, A, E>(&self, f: F) -> TransactionResult<A, E>
    where
        F: Fn(&TransactionalTree<K, V>) -> ConflictableTransactionResult<A, E>,
    {
        self.inner.transaction(|sled_transactional_tree| {
            f(&TransactionalTree::new(sled_transactional_tree))
        })
    }

    /// Create a new batched update that can be atomically applied.
    ///
    /// It is possible to apply a Batch in a transaction as well,
    /// which is the way you can apply a Batch to multiple Trees
    /// atomically.
    pub fn apply_batch(&self, batch: Batch<K, V>) -> Result<()> {
        Ok(self.inner.apply_batch(batch.inner)?)
    }

    /// Retrieve a value from the Tree if it exists.
    pub fn get(&self, key: &K) -> Result<Option<V>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .get(serialize(key)?)?
            .map(|v| deserialize(&v))
            .transpose()?)
    }

    /// Retrieve a value from the Tree if it exists. The key must be in serialized form.
    pub fn get_from_raw<B: AsRef<[u8]>>(&self, key_bytes: B) -> Result<Option<V>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .get(key_bytes.as_ref())?
            .map(|v| deserialize(&v))
            .transpose()?)
    }

    /// Deserialize a key and retrieve it's value from the Tree if it exists.
    /// The deserialization is only done if a value was retrieved successfully.
    pub fn get_kv_from_raw<B: AsRef<[u8]>>(&self, key_bytes: B) -> Result<Option<(K, V)>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .get(key_bytes.as_ref())?
            .map(|v| Ok::<_, anyhow::Error>((deserialize(key_bytes.as_ref())?, deserialize(&v)?)))
            .transpose()?)
    }

    /// Delete a value, returning the old value if it existed.
    pub fn remove(&self, key: &K) -> Result<Option<V>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .remove(serialize(key)?)?
            .map(|v| deserialize(&v))
            .transpose()?)
    }

    /// Compare and swap. Capable of unique creation, conditional
    /// modification, or deletion. If old is None, this will only set
    /// the value if it doesn't exist yet. If new is None, will delete
    /// the value if old is correct. If both old and new are Some,
    /// will modify the value if old is correct.
    ///
    /// It returns Ok(Ok(())) if operation finishes successfully.
    ///
    /// If it fails it returns: - Ok(Err(CompareAndSwapError(current,
    /// proposed))) if operation failed to setup a new
    /// value. CompareAndSwapError contains current and proposed
    /// values. - Err(Error::Unsupported) if the database is opened in
    /// read-only mode.
    pub fn compare_and_swap(
        &self,
        key: &K,
        old: Option<&V>,
        new: Option<&V>,
    ) -> Result<std::result::Result<(), CompareAndSwapError<V>>>
    where
        K: KV,
        V: KV,
    {
        let res = self.inner.compare_and_swap(
            serialize(key)?,
            old.map(|old| serialize(old)).transpose()?,
            new.map(|new| serialize(new)).transpose()?,
        )?;
        match res {
            Ok(()) => Ok(Ok(())),
            Err(cas_err) => Ok(Err(CompareAndSwapError {
                current: cas_err
                    .current
                    .as_ref()
                    .map(|b| deserialize(b))
                    .transpose()?,
                proposed: cas_err
                    .proposed
                    .as_ref()
                    .map(|b| deserialize(b))
                    .transpose()?,
            })),
        }
    }

    /// Fetch the value, apply a function to it and return the result.
    /// If serialization or deserialization of the value passed to or
    /// received from F fails the value in the database will not be
    /// changed and an error will be returned.
    pub fn update_and_fetch<F>(&self, key: &K, mut f: F) -> Result<Option<V>>
    where
        K: KV,
        V: KV,
        F: FnMut(Option<V>) -> Option<V>,
    {
        let mut failed = None;
        let res = self
            .inner
            .update_and_fetch(serialize(&key)?, |opt_value| {
                let v_ser = match opt_value.map(|v| deserialize(v)).transpose() {
                    Ok(v_ser) => v_ser,
                    Err(e) => {
                        failed = Some(anyhow::Error::from(e));
                        return opt_value.map(Vec::from);
                    }
                };
                match f(v_ser).map(|r| serialize(&r)).transpose() {
                    Ok(r_ser) => r_ser,
                    Err(e) => {
                        failed = Some(anyhow::Error::from(e));
                        opt_value.map(Vec::from)
                    }
                }
            })?
            .map(|v| deserialize(&v))
            .transpose()?;
        match failed {
            None => Ok(res),
            Some(e) => Err(e),
        }
    }

    /// Fetch the value, apply a function to it and return the previous value.
    pub fn fetch_and_update<F>(&self, key: &K, mut f: F) -> Result<Option<V>>
    where
        K: KV,
        V: KV,
        F: FnMut(Option<V>) -> Option<V>,
    {
        let mut failed = None;
        let res = self
            .inner
            .fetch_and_update(serialize(&key)?, |opt_value| {
                let v_ser = match opt_value.map(|v| deserialize(v)).transpose() {
                    Ok(v_ser) => v_ser,
                    Err(e) => {
                        failed = Some(anyhow::Error::from(e));
                        return opt_value.map(Vec::from);
                    }
                };
                match f(v_ser).map(|r| serialize(&r)).transpose() {
                    Ok(r_ser) => r_ser,
                    Err(e) => {
                        failed = Some(anyhow::Error::from(e));
                        opt_value.map(Vec::from)
                    }
                }
            })?
            .map(|v| deserialize(&v))
            .transpose()?;
        match failed {
            None => Ok(res),
            Some(e) => Err(e),
        }
    }

    /// Subscribe to `Event`s that happen to keys that have
    /// the specified prefix. Events for particular keys are
    /// guaranteed to be witnessed in the same order by all
    /// threads, but threads may witness different interleavings
    /// of `Event`s across different keys. If subscribers don't
    /// keep up with new writes, they will cause new writes
    /// to block. There is a buffer of 1024 items per
    /// `Subscriber`. This can be used to build reactive
    /// and replicated systems.
    pub fn watch_prefix<P>(&self, prefix: &P) -> Result<Subscriber<K, V>>
    where
        P: KV,
        K: Prefix<P>,
    {
        Ok(Subscriber::from_sled(
            self.inner.watch_prefix(serialize(prefix)?),
        ))
    }

    /// Subscribe to  all`Event`s. Events for particular keys are
    /// guaranteed to be witnessed in the same order by all
    /// threads, but threads may witness different interleavings
    /// of `Event`s across different keys. If subscribers don't
    /// keep up with new writes, they will cause new writes
    /// to block. There is a buffer of 1024 items per
    /// `Subscriber`. This can be used to build reactive
    /// and replicated systems.
    pub fn watch_all(&self) -> Subscriber<K, V>
    where
        K: KV,
    {
        Subscriber::from_sled(self.inner.watch_prefix(vec![]))
    }

    /// Synchronously flushes all dirty IO buffers and calls
    /// fsync. If this succeeds, it is guaranteed that all
    /// previous writes will be recovered if the system
    /// crashes. Returns the number of bytes flushed during
    /// this call.
    ///
    /// Flushing can take quite a lot of time, and you should
    /// measure the performance impact of using it on
    /// realistic sustained workloads running on realistic
    /// hardware.
    pub fn flush(&self) -> Result<usize> {
        Ok(self.inner.flush()?)
    }

    /// Asynchronously flushes all dirty IO buffers
    /// and calls fsync. If this succeeds, it is
    /// guaranteed that all previous writes will
    /// be recovered if the system crashes. Returns
    /// the number of bytes flushed during this call.
    ///
    /// Flushing can take quite a lot of time, and you
    /// should measure the performance impact of
    /// using it on realistic sustained workloads
    /// running on realistic hardware.
    pub async fn flush_async(&self) -> Result<usize> {
        Ok(self.inner.flush_async().await?)
    }

    /// Returns `true` if the `Tree` contains a value for
    /// the specified key.
    pub fn contains_key(&self, key: &K) -> Result<bool>
    where
        K: KV,
    {
        Ok(self.inner.contains_key(serialize(key)?)?)
    }

    /// Retrieve the key and value before the provided key,
    /// if one exists.
    pub fn get_lt(&self, key: &K) -> Result<Option<(K, V)>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .get_lt(serialize(key)?)?
            .map(|(k, v)| Ok::<_, anyhow::Error>((deserialize(&k)?, deserialize(&v)?)))
            .transpose()?)
    }

    /// Retrieve the next key and value from the `Tree` after the
    /// provided key.
    pub fn get_gt(&self, key: &K) -> Result<Option<(K, V)>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .get_gt(serialize(key)?)?
            .map(|(k, v)| Ok::<_, anyhow::Error>((deserialize(&k)?, deserialize(&v)?)))
            .transpose()?)
    }

    /// Merge state directly into a given key's value using the
    /// configured merge operator. This allows state to be written
    /// into a value directly, without any read-modify-write steps.
    /// Merge operators can be used to implement arbitrary data
    /// structures.
    ///
    /// Calling `merge` will return an `Unsupported` error if it
    /// is called without first setting a merge operator function.
    ///
    /// Merge operators are shared by all instances of a particular
    /// `Tree`. Different merge operators may be set on different
    /// `Tree`s.
    pub fn merge(&self, key: &K, value: &V) -> Result<Option<V>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .merge(serialize(key)?, serialize(value)?)?
            .map(|old_v| deserialize(&old_v))
            .transpose()?)
    }

    /// Sets a merge operator for use with the `merge` function.
    ///
    /// Merge state directly into a given key's value using the
    /// configured merge operator. This allows state to be written
    /// into a value directly, without any read-modify-write steps.
    /// Merge operators can be used to implement arbitrary data
    /// structures.
    ///
    /// # Panics
    ///
    /// Calling `merge` will panic if no merge operator has been
    /// configured.
    ///
    /// If serialization fails in the merge operator `merge` will panic
    pub fn set_merge_operator(&self, merge_operator: impl MergeOperator<K, V> + 'static)
    where
        K: KV,
        V: KV,
    {
        self.inner
            .set_merge_operator(move |key: &[u8], old_v: Option<&[u8]>, value: &[u8]| {
                let key_des = deserialize(key).expect("merge key deserialization failed");
                let old_v_des =
                    old_v.map(|v| deserialize(v).expect("merge old value deserialization failed"));
                let value_des = deserialize(value).expect("merge value deserialization failed");
                let res = merge_operator(key_des, old_v_des, value_des);
                res.map(|v| serialize(&v).expect("merge value serialization failed"))
            });
    }

    /// Create a double-ended iterator over the tuples of keys and
    /// values in this tree.
    pub fn iter(&self) -> Iter<K, V> {
        Iter::from_sled(self.inner.iter())
    }

    /// Create a double-ended iterator over tuples of keys and values,
    /// where the keys fall within the specified range.
    pub fn range<R: RangeBounds<K>>(&self, range: R) -> Result<Iter<K, V>>
    where
        K: KV + std::fmt::Debug,
    {
        match (range.start_bound(), range.end_bound()) {
            (Bound::Unbounded, Bound::Unbounded) => {
                Ok(Iter::from_sled(self.inner.range::<&[u8], _>(..)))
            }
            (Bound::Unbounded, Bound::Excluded(b)) => {
                Ok(Iter::from_sled(self.inner.range(..serialize(b)?)))
            }
            (Bound::Unbounded, Bound::Included(b)) => {
                Ok(Iter::from_sled(self.inner.range(..=serialize(b)?)))
            }
            // FIX: This is not excluding lower bound.
            (Bound::Excluded(b), Bound::Unbounded) => {
                Ok(Iter::from_sled(self.inner.range(serialize(b)?..)))
            }
            (Bound::Excluded(b), Bound::Excluded(bb)) => Ok(Iter::from_sled(
                self.inner.range(serialize(b)?..serialize(bb)?),
            )),
            (Bound::Excluded(b), Bound::Included(bb)) => Ok(Iter::from_sled(
                self.inner.range(serialize(b)?..=serialize(bb)?),
            )),
            (Bound::Included(b), Bound::Unbounded) => {
                Ok(Iter::from_sled(self.inner.range(serialize(b)?..)))
            }
            (Bound::Included(b), Bound::Excluded(bb)) => Ok(Iter::from_sled(
                self.inner.range(serialize(b)?..serialize(bb)?),
            )),
            (Bound::Included(b), Bound::Included(bb)) => Ok(Iter::from_sled(
                self.inner.range(serialize(b)?..=serialize(bb)?),
            )),
        }
    }

    /// Create an iterator over tuples of keys and values,
    /// where the all the keys starts with the given prefix.
    pub fn scan_prefix<P>(&self, prefix: &P) -> Result<Iter<K, V>>
    where
        P: KV,
        K: Prefix<P>,
    {
        Ok(Iter::from_sled(self.inner.scan_prefix(serialize(prefix)?)))
    }

    /// Returns the first key and value in the `Tree`, or
    /// `None` if the `Tree` is empty.
    pub fn first(&self) -> Result<Option<(K, V)>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .first()?
            .map(|(k, v)| Ok::<_, anyhow::Error>((deserialize(&k)?, deserialize(&v)?)))
            .transpose()?)
    }

    /// Returns the last key and value in the `Tree`, or
    /// `None` if the `Tree` is empty.
    pub fn last(&self) -> Result<Option<(K, V)>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .last()?
            .map(|(k, v)| Ok::<_, anyhow::Error>((deserialize(&k)?, deserialize(&v)?)))
            .transpose()?)
    }

    /// Atomically removes the maximum item in the `Tree` instance.
    pub fn pop_max(&self) -> Result<Option<(K, V)>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .pop_max()?
            .map(|(k, v)| Ok::<_, anyhow::Error>((deserialize(&k)?, deserialize(&v)?)))
            .transpose()?)
    }

    /// Atomically removes the minimum item in the `Tree` instance.
    pub fn pop_min(&self) -> Result<Option<(K, V)>>
    where
        K: KV,
        V: KV,
    {
        Ok(self
            .inner
            .pop_min()?
            .map(|(k, v)| Ok::<_, anyhow::Error>((deserialize(&k)?, deserialize(&v)?)))
            .transpose()?)
    }

    /// Returns the number of elements in this tree.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the `Tree` contains no elements.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clears the `Tree`, removing all values.
    ///
    /// Note that this is not atomic.
    pub fn clear(&self) -> Result<()> {
        Ok(self.inner.clear()?)
    }

    /// Returns the name of the tree.
    pub fn name(&self) -> IVec {
        self.inner.name()
    }

    /// Returns the CRC32 of all keys and values
    /// in this Tree.
    ///
    /// This is O(N) and locks the underlying tree
    /// for the duration of the entire scan.
    pub fn checksum(&self) -> Result<u32> {
        Ok(self.inner.checksum()?)
    }
}

/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use sled::{Config, IVec};
///
/// fn concatenate_merge(
///   _key: String,               // the key being merged
///   old_value: Option<Vec<f32>>,  // the previous value, if one existed
///   merged_bytes: Vec<f32>        // the new bytes being merged in
/// ) -> Option<Vec<f32>> {       // set the new value, return None to delete
///   let mut ret = old_value
///     .map(|ov| ov.to_vec())
///     .unwrap_or_else(|| vec![]);
///
///   ret.extend_from_slice(&merged_bytes);
///
///   Some(ret)
/// }
///
/// let db = sled::Config::new()
///   .temporary(true).open()?;
///
/// let tree = typed_sled::Tree::<String, Vec<f32>>::open(&db, "unique_id");
/// tree.set_merge_operator(concatenate_merge);
///
/// let k = String::from("some_key");
///
/// tree.insert(&k, &vec![0.0]);
/// tree.merge(&k, &vec![1.0]);
/// tree.merge(&k, &vec![2.0]);
/// assert_eq!(tree.get(&k)?, Some(vec![0.0, 1.0, 2.0]));
///
/// // Replace previously merged data. The merge function will not be called.
/// tree.insert(&k, &vec![3.0]);
/// assert_eq!(tree.get(&k)?, Some(vec![3.0]));
///
/// // Merges on non-present values will cause the merge function to be called
/// // with `old_value == None`. If the merge function returns something (which it
/// // does, in this case) a new value will be inserted.
/// tree.remove(&k);
/// tree.merge(&k, &vec![4.0]);
/// assert_eq!(tree.get(&k)?, Some(vec![4.0]));
/// # Ok(()) }
/// ```
pub trait MergeOperator<K, V>: Fn(K, Option<V>, V) -> Option<V> {}

impl<K, V, F> MergeOperator<K, V> for F where F: Fn(K, Option<V>, V) -> Option<V> {}

pub struct Iter<K, V> {
    inner: sled::Iter,
    _key: PhantomData<fn() -> K>,
    _value: PhantomData<fn() -> V>,
}

impl<K: KV, V: KV> Iterator for Iter<K, V> {
    type Item = Result<(K, V)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|r| match r {
            Ok((k, v)) => Ok((deserialize(&k)?, deserialize(&v)?)),
            Err(e) => Err(anyhow::Error::from(e)),
        })
    }

    fn last(mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|r| match r {
            Ok((k, v)) => Ok((deserialize(&k)?, deserialize(&v)?)),
            Err(e) => Err(anyhow::Error::from(e)),
        })
    }
}

impl<K: KV, V: KV> DoubleEndedIterator for Iter<K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|r| match r {
            Ok((k, v)) => Ok((deserialize(&k)?, deserialize(&v)?)),
            Err(e) => Err(anyhow::Error::from(e)),
        })
    }
}

impl<K, V> Iter<K, V> {
    pub fn from_sled(iter: sled::Iter) -> Self {
        Iter {
            inner: iter,
            _key: PhantomData,
            _value: PhantomData,
        }
    }

    pub fn keys(self) -> impl DoubleEndedIterator<Item = Result<K>> + Send + Sync
    where
        K: KV + Send + Sync,
        V: KV + Send + Sync,
    {
        self.map(|r| r.map(|(k, _v)| k))
    }

    /// Iterate over the values of this Tree
    pub fn values(self) -> impl DoubleEndedIterator<Item = Result<V>> + Send + Sync
    where
        K: KV + Send + Sync,
        V: KV + Send + Sync,
    {
        self.map(|r| r.map(|(_k, v)| v))
    }
}

#[derive(Clone, Debug)]
pub struct Batch<K, V> {
    inner: sled::Batch,
    _key: PhantomData<K>,
    _value: PhantomData<V>,
}

impl<K, V> Batch<K, V> {
    pub fn insert(&mut self, key: &K, value: &V) -> Result<()>
    where
        K: KV,
        V: KV,
    {
        Ok(self.inner.insert(serialize(key)?, serialize(value)?))
    }

    pub fn remove(&mut self, key: &K) -> Result<()>
    where
        K: KV,
    {
        Ok(self.inner.remove(serialize(key)?))
    }
}

// Implementing Default manually to not require K and V to implement Default.
impl<K, V> Default for Batch<K, V> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            _key: PhantomData,
            _value: PhantomData,
        }
    }
}

use pin_project::pin_project;
#[pin_project]
pub struct Subscriber<K, V> {
    #[pin]
    inner: sled::Subscriber,
    _key: PhantomData<fn() -> K>,
    _value: PhantomData<fn() -> V>,
}

impl<K, V> Subscriber<K, V> {
    pub fn next_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> std::result::Result<Result<Event<K, V>>, std::sync::mpsc::RecvTimeoutError>
    where
        K: KV,
        V: KV,
    {
        self.inner
            .next_timeout(timeout)
            .map(|e| Event::from_sled(&e))
    }

    pub fn from_sled(subscriber: sled::Subscriber) -> Self {
        Self {
            inner: subscriber,
            _key: PhantomData,
            _value: PhantomData,
        }
    }
}

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

impl<K: KV + Unpin, V: KV + Unpin> Future for Subscriber<K, V> {
    type Output = Option<Result<Event<K, V>>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project()
            .inner
            .poll(cx)
            .map(|opt| opt.map(|e| Event::from_sled(&e)))
    }
}

impl<K: KV, V: KV> Iterator for Subscriber<K, V> {
    type Item = Result<Event<K, V>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|e| Event::from_sled(&e))
    }
}

pub enum Event<K, V> {
    Insert { key: K, value: V },
    Remove { key: K },
}

impl<K, V> Event<K, V> {
    pub fn key(&self) -> &K
    where
        K: KV,
    {
        match self {
            Self::Insert { key, .. } | Self::Remove { key } => key,
        }
    }

    pub fn from_sled(event: &sled::Event) -> Result<Self>
    where
        K: KV,
        V: KV,
    {
        match event {
            sled::Event::Insert { key, value } => Ok(Self::Insert {
                key: deserialize(key)?,
                value: deserialize(value)?,
            }),
            sled::Event::Remove { key } => Ok(Self::Remove {
                key: deserialize(key)?,
            }),
        }
    }
}

/// The function which is used to deserialize all keys and values.
pub fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T>
where
    T: serde::de::Deserialize<'a>,
{
    Ok(bincode::DefaultOptions::new()
        .with_big_endian()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .deserialize(bytes)?)
}

/// The function which is used to serialize all keys and values.
pub fn serialize<T>(value: &T) -> Result<Vec<u8>>
where
    T: serde::Serialize,
{
    Ok(bincode::DefaultOptions::new()
        .with_big_endian()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .serialize(value)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range() {
        let config = sled::Config::new().temporary(true);
        let db = config.open().unwrap();

        let tree: Tree<u32, u32> = Tree::open(&db, "test_tree");

        tree.insert(&1, &2).unwrap();
        tree.insert(&3, &4).unwrap();
        tree.insert(&6, &2).unwrap();
        tree.insert(&10, &2).unwrap();
        tree.insert(&15, &2).unwrap();
        tree.flush().unwrap();

        let expect_results = [(6, 2), (10, 2)];

        for (i, result) in tree.range(6..11).unwrap().enumerate() {
            assert_eq!(result.unwrap(), expect_results[i]);
        }
    }

    #[test]
    fn test_cas() {
        let config = sled::Config::new().temporary(true);
        let db = config.open().unwrap();

        let tree: Tree<u32, u32> = Tree::open(&db, "test_tree");

        let current = 2;
        tree.insert(&1, &current).unwrap();
        let expected = 3;
        let proposed = 4;
        let res = tree
            .compare_and_swap(&1, Some(&expected), Some(&proposed))
            .expect("db failure");

        assert_eq!(
            res,
            Err(CompareAndSwapError {
                current: Some(current),
                proposed: Some(proposed),
            }),
        );
    }
}
