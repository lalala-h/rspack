use rkyv::{
  vec::{ArchivedVec, VecResolver},
  with::{ArchiveWith, DeserializeWith, SerializeWith},
};

use crate::{CacheableDeserializer, CacheableSerializer, DeserializeError, SerializeError};

pub struct AsDyn;

pub trait AsDynConverter {
  type Context;
  fn to_bytes(&self, context: &mut Self::Context) -> Result<Vec<u8>, SerializeError>;
  fn from_bytes(s: &[u8], context: &mut Self::Context) -> Result<Self, DeserializeError>
  where
    Self: Sized;
}

pub struct AsCacheableResolver {
  inner: VecResolver,
  len: usize,
}

impl<T, C> ArchiveWith<T> for AsDyn
where
  T: AsDynConverter<Context = C>,
{
  type Archived = ArchivedVec<u8>;
  type Resolver = AsCacheableResolver;

  #[inline]
  unsafe fn resolve_with(
    _field: &T,
    pos: usize,
    resolver: Self::Resolver,
    out: *mut Self::Archived,
  ) {
    ArchivedVec::resolve_from_len(resolver.len, pos, resolver.inner, out)
  }
}

impl<'a, T, C> SerializeWith<T, CacheableSerializer<'a, C>> for AsDyn
where
  T: AsDynConverter<Context = C>,
{
  #[inline]
  fn serialize_with(
    field: &T,
    serializer: &mut CacheableSerializer<'a, C>,
  ) -> Result<Self::Resolver, SerializeError> {
    let bytes = &field.to_bytes(serializer.get_context())?;
    Ok(AsCacheableResolver {
      inner: ArchivedVec::serialize_from_slice(bytes, serializer)?,
      len: bytes.len(),
    })
  }
}

impl<'a, T, C> DeserializeWith<ArchivedVec<u8>, T, CacheableDeserializer<'a, C>> for AsDyn
where
  T: AsDynConverter<Context = C>,
{
  #[inline]
  fn deserialize_with(
    field: &ArchivedVec<u8>,
    de: &mut CacheableDeserializer<'a, C>,
  ) -> Result<T, DeserializeError> {
    AsDynConverter::from_bytes(field, de.get_context())
  }
}

// for rspack_source
/*use std::sync::Arc;

use rspack_sources::RawSource;
impl Cacheable for rspack_sources::BoxSource {
  fn serialize(&self) -> Vec<u8> {
    let inner = self.as_ref().as_any();
    let mut data: Option<CacheableDynData> = None;
    if let Some(raw_source) = inner.downcast_ref::<rspack_sources::RawSource>() {
      match raw_source {
        RawSource::Buffer(buf) => {
          // TODO try avoid clone
          data = Some(CacheableDynData(
            String::from("RawSource::Buffer"),
            buf.clone(),
          ));
        }
        RawSource::Source(source) => {
          data = Some(CacheableDynData(
            String::from("RawSource::Source"),
            source.as_bytes().to_vec(),
          ));
        }
      }
      //    } else if let Some() = inner.downcast_ref::<rspack_sources::RawSource>() {
    }

    if let Some(data) = data {
      to_bytes(&data)
    } else {
      panic!("unsupport box source")
    }
  }
  fn deserialize(bytes: &[u8]) -> Self
  where
    Self: Sized,
  {
    let CacheableDynData(type_name, data) = from_bytes(bytes);
    match type_name.as_str() {
      "RawSource::Buffer" => Arc::new(RawSource::Buffer(data)),
      "RawSource::Source" => Arc::new(RawSource::Source(
        String::from_utf8(data).expect("convert to string failed"),
      )),
      _ => {
        panic!("unsupport box source")
      }
    }
  }
}*/
