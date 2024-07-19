use core::{alloc::Layout, ptr::NonNull};

use rkyv::{
  ser::{
    serializers::{
      AlignedSerializer, AllocScratch, FallbackScratch, HeapScratch, SharedSerializeMap,
    },
    ScratchSpace, Serializer, SharedSerializeRegistry,
  },
  AlignedVec, Archive, ArchiveUnsized, Fallible, Serialize,
};

#[derive(Debug)]
pub enum SerializeError {
  /// An error occurred while serializing
  SerializerError(<AlignedSerializer<AlignedVec> as Fallible>::Error),
  /// An error occurred while using scratch space
  ScratchSpaceError(<AllocScratch as Fallible>::Error),
  /// An error occurred while serializing shared memory
  SharedError(<SharedSerializeMap as Fallible>::Error),
}

pub struct CacheableSerializer<'a, C> {
  serializer: AlignedSerializer<AlignedVec>,
  scratch: FallbackScratch<HeapScratch<1024>, AllocScratch>,
  shared: SharedSerializeMap,
  context: &'a mut C,
}

impl<'a, C> CacheableSerializer<'a, C> {
  fn new(context: &'a mut C) -> Self {
    Self {
      serializer: Default::default(),
      scratch: Default::default(),
      shared: Default::default(),
      context,
    }
  }
  pub fn get_context(&mut self) -> &mut C {
    self.context
  }
}

impl<C> Fallible for CacheableSerializer<'_, C> {
  type Error = SerializeError;
}

impl<C> Serializer for CacheableSerializer<'_, C> {
  #[inline]
  fn pos(&self) -> usize {
    self.serializer.pos()
  }

  #[inline]
  fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
    self
      .serializer
      .write(bytes)
      .map_err(SerializeError::SerializerError)
  }

  #[inline]
  fn pad(&mut self, padding: usize) -> Result<(), Self::Error> {
    self
      .serializer
      .pad(padding)
      .map_err(SerializeError::SerializerError)
  }

  #[inline]
  fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
    self
      .serializer
      .align(align)
      .map_err(SerializeError::SerializerError)
  }

  #[inline]
  fn align_for<T>(&mut self) -> Result<usize, Self::Error> {
    self
      .serializer
      .align_for::<T>()
      .map_err(SerializeError::SerializerError)
  }

  #[inline]
  unsafe fn resolve_aligned<T: Archive + ?Sized>(
    &mut self,
    value: &T,
    resolver: T::Resolver,
  ) -> Result<usize, Self::Error> {
    self
      .serializer
      .resolve_aligned::<T>(value, resolver)
      .map_err(SerializeError::SerializerError)
  }

  #[inline]
  unsafe fn resolve_unsized_aligned<T: ArchiveUnsized + ?Sized>(
    &mut self,
    value: &T,
    to: usize,
    metadata_resolver: T::MetadataResolver,
  ) -> Result<usize, Self::Error> {
    self
      .serializer
      .resolve_unsized_aligned(value, to, metadata_resolver)
      .map_err(SerializeError::SerializerError)
  }
}

impl<C> ScratchSpace for CacheableSerializer<'_, C> {
  #[inline]
  unsafe fn push_scratch(&mut self, layout: Layout) -> Result<NonNull<[u8]>, Self::Error> {
    self
      .scratch
      .push_scratch(layout)
      .map_err(SerializeError::ScratchSpaceError)
  }

  #[inline]
  unsafe fn pop_scratch(&mut self, ptr: NonNull<u8>, layout: Layout) -> Result<(), Self::Error> {
    self
      .scratch
      .pop_scratch(ptr, layout)
      .map_err(SerializeError::ScratchSpaceError)
  }
}

impl<C> SharedSerializeRegistry for CacheableSerializer<'_, C> {
  #[inline]
  fn get_shared_ptr(&self, value: *const u8) -> Option<usize> {
    self.shared.get_shared_ptr(value)
  }

  #[inline]
  fn add_shared_ptr(&mut self, value: *const u8, pos: usize) -> Result<(), Self::Error> {
    self
      .shared
      .add_shared_ptr(value, pos)
      .map_err(SerializeError::SharedError)
  }
}

pub fn to_bytes<'a, T, C>(data: &'a T, ctx: &'a mut C) -> Result<Vec<u8>, SerializeError>
where
  T: Serialize<CacheableSerializer<'a, C>>,
{
  let mut serializer = CacheableSerializer::new(ctx);
  serializer.serialize_value(data)?;
  Ok(serializer.serializer.into_inner().to_vec())
}
