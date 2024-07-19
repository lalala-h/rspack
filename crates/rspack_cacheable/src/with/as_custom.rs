use rkyv::{
  ser::{ScratchSpace, Serializer},
  with::{ArchiveWith, DeserializeWith, SerializeWith},
  Archive, Deserialize, Fallible, Serialize,
};

pub struct AsCustom;

pub trait AsCustomConverter {
  type S;
  type Context;
  fn to(&self) -> Self::S;
  fn from(data: &Self::S, ctx: &mut Self::Context) -> Self
  where
    Self: Sized;
}

impl<T, O> ArchiveWith<T> for AsCustom
where
  T: AsCustomConverter<S = O>,
  O: Archive,
{
  type Archived = <O as Archive>::Archived;
  type Resolver = <O as Archive>::Resolver;

  unsafe fn resolve_with(
    field: &T,
    pos: usize,
    resolver: Self::Resolver,
    out: *mut Self::Archived,
  ) {
    <O as Archive>::resolve(&field.to(), pos, resolver, out)
  }
}

impl<T, O, S> SerializeWith<T, S> for AsCustom
where
  T: AsCustomConverter<S = O>,
  O: Archive + Serialize<S>,
  S: Fallible + ScratchSpace + Serializer + ?Sized,
{
  fn serialize_with(field: &T, s: &mut S) -> Result<Self::Resolver, S::Error> {
    <O as Serialize<S>>::serialize(&field.to(), s)
  }
}

impl<T, O, D> DeserializeWith<<O as Archive>::Archived, T, D> for AsCustom
where
  T: AsCustomConverter<S = O, Context = D>,
  O: Archive,
  O::Archived: Deserialize<O, D>,
  D: Fallible + ?Sized,
{
  fn deserialize_with(field: &<O as Archive>::Archived, d: &mut D) -> Result<T, D::Error> {
    let data = <O::Archived as Deserialize<O, D>>::deserialize(field, d)?;
    Ok(AsCustomConverter::from(&data, d))
  }
}
