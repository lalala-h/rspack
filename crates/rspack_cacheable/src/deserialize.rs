use rkyv::{
  check_archived_root,
  de::{deserializers::SharedDeserializeMap, SharedDeserializeRegistry, SharedPointer},
  validation::validators::DefaultValidator,
  Archive, CheckBytes, Deserialize, Fallible,
};

#[derive(Debug)]
pub enum DeserializeError {
  /// A validation error occurred.
  CheckBytesError,
  /// A shared pointer was added multiple times
  DuplicateSharedPointer,
  //  DeserializeFailed(String),
}

pub struct CacheableDeserializer<'a, C> {
  shared: SharedDeserializeMap,
  context: &'a mut C,
}

impl<'a, C> CacheableDeserializer<'a, C> {
  fn new(context: &'a mut C) -> Self {
    Self {
      shared: SharedDeserializeMap::default(),
      context,
    }
  }

  pub fn get_context(&mut self) -> &mut C {
    self.context
  }
}

impl<C> Fallible for CacheableDeserializer<'_, C> {
  type Error = DeserializeError;
}

impl<C> SharedDeserializeRegistry for CacheableDeserializer<'_, C> {
  fn get_shared_ptr(&mut self, ptr: *const u8) -> Option<&dyn SharedPointer> {
    self.shared.get_shared_ptr(ptr)
  }

  fn add_shared_ptr(
    &mut self,
    ptr: *const u8,
    shared: Box<dyn SharedPointer>,
  ) -> Result<(), Self::Error> {
    self
      .shared
      .add_shared_ptr(ptr, shared)
      .map_err(|_| DeserializeError::DuplicateSharedPointer)
  }
}

pub fn from_bytes<'a, T, C>(bytes: &'a [u8], context: &'a mut C) -> Result<T, DeserializeError>
where
  T: Archive,
  T::Archived: 'a + CheckBytes<DefaultValidator<'a>> + Deserialize<T, CacheableDeserializer<'a, C>>,
{
  let mut deserializer = CacheableDeserializer::new(context);
  check_archived_root::<T>(bytes)
    .map_err(|_| DeserializeError::CheckBytesError)?
    .deserialize(&mut deserializer)
}
