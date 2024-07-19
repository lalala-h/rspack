use rkyv::{
  ser::{ScratchSpace, Serializer},
  string::{ArchivedString, StringResolver},
  with::{ArchiveWith, DeserializeWith, SerializeWith},
  Fallible,
};

pub struct AsRefStr;

pub trait AsRefStrConverter {
  fn as_str(&self) -> &str;
  fn from_str(s: &str) -> Self
  where
    Self: Sized;
}

impl<T> ArchiveWith<T> for AsRefStr
where
  T: AsRefStrConverter,
{
  type Archived = ArchivedString;
  type Resolver = StringResolver;

  #[inline]
  unsafe fn resolve_with(
    field: &T,
    pos: usize,
    resolver: Self::Resolver,
    out: *mut Self::Archived,
  ) {
    ArchivedString::resolve_from_str(field.as_str(), pos, resolver, out);
  }
}

impl<T, S> SerializeWith<T, S> for AsRefStr
where
  T: AsRefStrConverter,
  S: ?Sized + Serializer + ScratchSpace,
{
  #[inline]
  fn serialize_with(field: &T, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
    ArchivedString::serialize_from_str(field.as_str(), serializer)
  }
}

impl<T, D> DeserializeWith<ArchivedString, T, D> for AsRefStr
where
  T: AsRefStrConverter,
  D: ?Sized + Fallible,
{
  #[inline]
  fn deserialize_with(field: &ArchivedString, _: &mut D) -> Result<T, D::Error> {
    Ok(AsRefStrConverter::from_str(field.as_str()))
  }
}

// for swc_core atom
impl AsRefStrConverter for swc_core::ecma::atoms::Atom {
  fn as_str(&self) -> &str {
    self.as_str()
  }
  fn from_str(s: &str) -> Self
  where
    Self: Sized,
  {
    Self::from(s)
  }
}
