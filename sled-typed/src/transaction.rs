use crate::{deserialize, serialize, Batch, Tree, KV};
use sled::transaction::{
    ConflictableTransactionError, ConflictableTransactionResult, TransactionResult,
};
use std::{marker::PhantomData, result::Result as Res};

pub struct TransactionalTree<'a, K, V> {
    inner: &'a sled::transaction::TransactionalTree,
    _key: PhantomData<fn() -> K>,
    _value: PhantomData<fn() -> V>,
}

impl<'a, K, V> TransactionalTree<'a, K, V> {
    pub(crate) fn new(sled: &'a sled::transaction::TransactionalTree) -> Self {
        Self {
            inner: sled,
            _key: PhantomData,
            _value: PhantomData,
        }
    }

    pub fn insert(
        &self,
        key: &K,
        value: &V,
    ) -> Res<Option<V>, ConflictableTransactionError<anyhow::Error>>
    where
        K: KV,
        V: KV,
    {
        let k_ser = serialize(key).map_err(|e| ConflictableTransactionError::Abort(e.into()))?;
        let v_ser = serialize(value).map_err(|e| ConflictableTransactionError::Abort(e.into()))?;
        Ok(self
            .inner
            .insert(k_ser, v_ser)?
            .map(|v| deserialize(&v).map_err(|e| ConflictableTransactionError::Abort(e.into())))
            .transpose()?)
    }

    pub fn remove(&self, key: &K) -> Res<Option<V>, ConflictableTransactionError<anyhow::Error>>
    where
        K: KV,
        V: KV,
    {
        let k_ser = serialize(key).map_err(|e| ConflictableTransactionError::Abort(e.into()))?;
        Ok(self
            .inner
            .remove(k_ser)?
            .map(|v| deserialize(&v).map_err(|e| ConflictableTransactionError::Abort(e.into())))
            .transpose()?)
    }

    pub fn get(&self, key: &K) -> Res<Option<V>, ConflictableTransactionError<anyhow::Error>>
    where
        K: KV,
        V: KV,
    {
        let k_ser = serialize(key).map_err(|e| ConflictableTransactionError::Abort(e.into()))?;
        Ok(self
            .inner
            .get(k_ser)?
            .map(|v| deserialize(&v).map_err(|e| ConflictableTransactionError::Abort(e.into())))
            .transpose()?)
    }

    pub fn apply_batch(
        &self,
        batch: &Batch<K, V>,
    ) -> std::result::Result<(), sled::transaction::UnabortableTransactionError> {
        self.inner.apply_batch(&batch.inner)
    }

    pub fn flush(&self) {
        self.inner.flush()
    }

    pub fn generate_id(&self) -> sled::Result<u64> {
        self.inner.generate_id()
    }
}

pub trait Transactional<E = ()> {
    type View<'a>;

    fn transaction<F, A>(&self, f: F) -> TransactionResult<A, E>
    where
        F: for<'a> Fn(Self::View<'a>) -> ConflictableTransactionResult<A, E>;
}

macro_rules! impl_transactional {
  ($($k:ident, $v:ident, $i:tt),+) => {
      impl<E, $($k, $v),+> Transactional<E> for ($(&Tree<$k, $v>),+) {
          type View<'a> = (
              $(TransactionalTree<'a, $k, $v>),+
          );

          fn transaction<F, A>(&self, f: F) -> TransactionResult<A, E>
          where
              F: for<'a> Fn(Self::View<'a>) -> ConflictableTransactionResult<A, E>,
          {
              use sled::Transactional;

              ($(&self.$i.inner),+).transaction(|trees| {
                  f((
                      $(TransactionalTree::new(&trees.$i)),+
                  ))
              })
          }
      }
  };
}

impl_transactional!(K0, V0, 0, K1, V1, 1);
impl_transactional!(K0, V0, 0, K1, V1, 1, K2, V2, 2);
impl_transactional!(K0, V0, 0, K1, V1, 1, K2, V2, 2, K3, V3, 3);
impl_transactional!(K0, V0, 0, K1, V1, 1, K2, V2, 2, K3, V3, 3, K4, V4, 4);
impl_transactional!(K0, V0, 0, K1, V1, 1, K2, V2, 2, K3, V3, 3, K4, V4, 4, K5, V5, 5);
impl_transactional!(K0, V0, 0, K1, V1, 1, K2, V2, 2, K3, V3, 3, K4, V4, 4, K5, V5, 5, K6, V6, 6);
impl_transactional!(
    K0, V0, 0, K1, V1, 1, K2, V2, 2, K3, V3, 3, K4, V4, 4, K5, V5, 5, K6, V6, 6, K7, V7, 7
);
impl_transactional!(
    K0, V0, 0, K1, V1, 1, K2, V2, 2, K3, V3, 3, K4, V4, 4, K5, V5, 5, K6, V6, 6, K7, V7, 7, K8, V8,
    8
);
impl_transactional!(
    K0, V0, 0, K1, V1, 1, K2, V2, 2, K3, V3, 3, K4, V4, 4, K5, V5, 5, K6, V6, 6, K7, V7, 7, K8, V8,
    8, K9, V9, 9
);
impl_transactional!(
    K0, V0, 0, K1, V1, 1, K2, V2, 2, K3, V3, 3, K4, V4, 4, K5, V5, 5, K6, V6, 6, K7, V7, 7, K8, V8,
    8, K9, V9, 9, K10, V10, 10
);

#[test]
fn test_multiple_tree_transaction() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let tree0 = Tree::<u32, i32>::open(&db, "tree0");
    let tree1 = Tree::<u16, i16>::open(&db, "tree1");
    let tree2 = Tree::<u8, i8>::open(&db, "tree2");
    (&tree0, &tree1, &tree2)
        .transaction(|(t0, t1, t2)| {
            t0.insert(&0, &0)?;
            t1.insert(&0, &0)?;
            t2.insert(&0, &0)?;
            Ok(())
        })
        .unwrap();
    assert_eq!(tree0.get(&0).unwrap(), Some(0));
    assert_eq!(tree1.get(&0).unwrap(), Some(0));
}
