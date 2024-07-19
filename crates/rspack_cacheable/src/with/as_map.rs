pub struct AsVec<T>;

impl<T, S> rkyv::with::SerializeWith<T, S> for AsVec
where
  T: AsStringConverter,
  S: ?Sized + rkyv::ser::Serializer + rkyv::ser::ScratchSpace,
{
  #[inline]
  fn serialize_with(field: &T, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
    rkyv::string::ArchivedString::serialize_from_str(&field.to_string(), serializer)
  }
}

impl<T> rkyv::with::ArchiveWith<T> for AsVec
where
  T: AsStringConverter,
{
  type Archived = rkyv::string::ArchivedString;
  type Resolver = rkyv::string::StringResolver;

  #[inline]
  unsafe fn resolve_with(
    field: &T,
    pos: usize,
    resolver: Self::Resolver,
    out: *mut Self::Archived,
  ) {
    rkyv::string::ArchivedString::resolve_from_str(&field.to_string(), pos, resolver, out);
  }
}

impl<T, D> rkyv::with::DeserializeWith<rkyv::string::ArchivedString, T, D> for AsVec
where
  T: AsStringConverter,
  D: ?Sized + rkyv::Fallible,
{
  #[inline]
  fn deserialize_with(field: &rkyv::string::ArchivedString, _: &mut D) -> Result<T, D::Error> {
    Ok(AsStringConverter::from_str(field.as_str()))
  }
}

// for pathbuf
impl AsStringConverter for std::path::PathBuf {
  fn to_string(&self) -> String {
    self.to_string_lossy().to_string()
  }
  fn from_str(s: &str) -> Self
  where
    Self: Sized,
  {
    std::path::PathBuf::from(s)
  }
}

// for json value
impl AsStringConverter for json::JsonValue {
  fn to_string(&self) -> String {
    json::stringify(self.clone())
  }
  fn from_str(s: &str) -> Self
  where
    Self: Sized,
  {
    json::parse(s).expect("parse json failed")
  }
}
