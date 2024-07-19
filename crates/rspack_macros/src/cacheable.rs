use proc_macro::TokenStream;
use quote::quote;
use syn::{
  parse::{Parse, ParseStream},
  parse_macro_input, Item, Result,
};

mod kw {
  syn::custom_keyword!(with);
}
pub struct CacheableArgs {
  pub with: syn::Path,
}
impl Parse for CacheableArgs {
  fn parse(input: ParseStream) -> Result<Self> {
    input.parse::<kw::with>()?;
    input.parse::<syn::Token![=]>()?;
    let with = input.parse::<syn::Path>()?;
    Ok(Self { with })
  }
}

pub fn impl_cacheable(tokens: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(tokens as Item);

  // add attr for some field
  match &mut input {
    Item::Enum(input) => {
      for v in input.variants.iter_mut() {
        for f in v.fields.iter_mut() {
          add_attr_for_field(f);
        }
      }
    }
    Item::Struct(input) => {
      for f in input.fields.iter_mut() {
        add_attr_for_field(f);
      }
    }
    _ => panic!("expect enum or struct"),
  }

  quote! {
      #[derive(
          rspack_cacheable::__private::rkyv::Archive,
          rspack_cacheable::__private::rkyv::Deserialize,
          rspack_cacheable::__private::rkyv::Serialize
      )]
      #[archive(check_bytes, crate="rspack_cacheable::__private::rkyv")]
      #input
  }
  .into()
}

pub fn impl_cacheable_with(tokens: TokenStream, with: syn::Path) -> TokenStream {
  let input = parse_macro_input!(tokens as Item);
  let (ident, _impl_generics, _ty_generics, _where_clause) = match &input {
    Item::Enum(input) => {
      let (a, b, c) = input.generics.split_for_impl();
      (&input.ident, a, b, c)
    }
    Item::Struct(input) => {
      let (a, b, c) = input.generics.split_for_impl();
      (&input.ident, a, b, c)
    }
    _ => panic!("expect enum or struct"),
  };
  let archived = quote! {<#with as rkyv::with::ArchiveWith<#ident>>::Archived};
  let resolver = quote! {<#with as rkyv::with::ArchiveWith<#ident>>::Resolver};
  let rkyv_with = quote! {rkyv::with::With<#ident, #with>};
  quote! {
      #input
      #[allow(non_upper_case_globals)]
      const _: () = {
          use rspack_cacheable::__private::rkyv;
          impl rkyv::Archive for #ident {
              type Archived = #archived;
              type Resolver = #resolver;
              #[inline]
              unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
                  <#rkyv_with>::cast(self).resolve(pos, resolver, out)
              }
          }
          impl<S> rkyv::Serialize<S> for #ident
          where
              #rkyv_with: rkyv::Serialize<S>,
              S: rkyv::Fallible + ?Sized,
          {
              #[inline]
              fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
                  <#rkyv_with>::cast(self).serialize(serializer)
              }
          }
          impl<D: rkyv::Fallible + ?Sized> rkyv::Deserialize<#ident, D> for #archived
          where
              #rkyv_with: rkyv::Archive,
              rkyv::Archived<#rkyv_with>: rkyv::Deserialize<#rkyv_with, D>,
          {
              #[inline]
              fn deserialize(&self, _deserializer: &mut D) -> Result<#ident, D::Error> {
                  Ok(
                      rkyv::Deserialize::<#rkyv_with, D>::deserialize(
                          self,
                          _deserializer,
                      )?.into_inner()
                  )
              }
          }
      };
  }
  .into()
}

fn add_attr_for_field(field: &mut syn::Field) {
  if let syn::Type::Path(ty_path) = &field.ty {
    if let Some(seg) = &ty_path.path.segments.last() {
      if seg.ident == "Box" {
        if let syn::PathArguments::AngleBracketed(arg) = &seg.arguments {
          if let Some(syn::GenericArgument::Type(syn::Type::TraitObject(_))) = &arg.args.first() {
            // for Box<dyn xxx>
            field.attrs.push(syn::parse_quote! {
                #[with(rspack_cacheable::with::AsDyn)]
            });
            return;
          }
        }
      }

      if seg.ident == "Option" {
        if let syn::PathArguments::AngleBracketed(arg) = &seg.arguments {
          if let Some(syn::GenericArgument::Type(syn::Type::Path(sub_path))) = &arg.args.last() {
            if let Some(seg) = sub_path.path.segments.last() {
              if seg.ident == "JsonValue" {
                // for Option<JsonValue>
                field.attrs.push(syn::parse_quote! {
                    #[with(rspack_cacheable::with::AsOption<rspack_cacheable::with::AsString>)]
                });
                return;
              }

              if seg.ident == "BoxSource" {
                // for Option<BoxSource>
                field.attrs.push(syn::parse_quote! {
                    #[with(rspack_cacheable::with::AsOption<rspack_cacheable::with::AsCacheable>)]
                });
                return;
              }
            }
          }
        }
      }

      if seg.ident == "HashSet" {
        if let syn::PathArguments::AngleBracketed(arg) = &seg.arguments {
          if let Some(syn::GenericArgument::Type(syn::Type::Path(sub_path))) = &arg.args.last() {
            if sub_path.path.is_ident("PathBuf") {
              // for HashSet<PathBuf>
              field.attrs.push(syn::parse_quote! {
                  #[with(rspack_cacheable::with::AsVec<rspack_cacheable::with::AsString>)]
              });
              return;
            }
            if sub_path.path.is_ident("Atom") {
              // for HashSet<Atom>
              field.attrs.push(syn::parse_quote! {
                  #[with(rspack_cacheable::with::AsVec<rspack_cacheable::with::AsRefStr>)]
              });
              return;
            }
          }
        }
      }

      if seg.ident == "BoxSource" {
        field.attrs.push(syn::parse_quote! {
            #[with(rspack_cacheable::with::AsCacheable)]
        });
        return;
      }

      if seg.ident == "RwLock" {
        // TODO
        field.attrs.push(syn::parse_quote! {
            #[with(rspack_cacheable::with::Skip)]
        });
        return;
      }
    }
  }
}
