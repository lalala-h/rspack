pub use rkyv::with::Skip;
use rkyv::{
  with::{ArchiveWith, DeserializeWith, SerializeWith},
  Fallible,
};

use crate::{CacheableDeserializer, DeserializeError};

pub struct SkipWithDeserialize;

trait SkipDeserialize<C> {
  fn deserialize(ctx: &mut C) -> Result<Self, DeserializeError>
  where
    Self: Sized;
}

impl<F> ArchiveWith<F> for SkipWithDeserialize {
  type Archived = ();
  type Resolver = ();

  unsafe fn resolve_with(_: &F, _: usize, _: Self::Resolver, _: *mut Self::Archived) {}
}

impl<F, S: Fallible + ?Sized> SerializeWith<F, S> for SkipWithDeserialize {
  fn serialize_with(_: &F, _: &mut S) -> Result<(), S::Error> {
    Ok(())
  }
}

impl<'a, F, C> DeserializeWith<(), F, CacheableDeserializer<'a, C>> for SkipWithDeserialize
where
  F: SkipDeserialize<C>,
{
  fn deserialize_with(_: &(), de: &mut CacheableDeserializer<C>) -> Result<F, DeserializeError> {
    F::deserialize(de.get_context())
  }
}
