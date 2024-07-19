use proc_macro::TokenStream;
use quote::quote;
use syn::{
  parse::{Parse, ParseStream},
  parse_quote, Ident, ItemImpl, ItemTrait, Result, Type,
};

pub struct CacheableDynArgs {
  context: syn::Path,
}
impl Parse for CacheableDynArgs {
  fn parse(input: ParseStream) -> Result<Self> {
    let context = input.parse::<syn::Path>()?;
    Ok(Self { context })
  }
}

pub fn impl_trait(args: CacheableDynArgs, mut input: ItemTrait) -> TokenStream {
  let context = &args.context;
  let trait_ident = &input.ident;
  let flag_ident = Ident::new(&format!("{trait_ident}Flag"), trait_ident.span());
  let flag_vis = &input.vis;

  //  input
  //    .supertraits
  //    .push(parse_quote!(rspack_cacheable::with::AsDynConverter));
  input.items.push(parse_quote! {
      #[doc(hidden)]
      fn __cacheable_dyn_type_name(&self) -> &'static str;
  });
  input.items.push(parse_quote! {
      #[doc(hidden)]
      fn __cacheable_dyn_to_data(&self, context: &mut #context) -> Result<Vec<u8>, rspack_cacheable::SerializeError>;
  });
  input.items.push(parse_quote! {
      #[doc(hidden)]
      fn __cacheable_dyn_from_data(bytes: &[u8], context: &mut #context) -> Result<Self, rspack_cacheable::DeserializeError> where Self: Sized;
  });

  quote! {
      #input

      #[allow(non_upper_case_globals)]
      const _: () = {
          use rspack_cacheable::__private::inventory;
          use rspack_cacheable::__private::once_cell;
          use rspack_cacheable::{with::AsDynConverter, DeserializeError, SerializeError};
          type DeserializeFn = fn(&[u8], &mut #context) -> Result<Box<dyn #trait_ident>, DeserializeError>;

          #flag_vis struct #flag_ident {
              name: &'static str,
              deserialize: DeserializeFn
          }
          inventory::collect!(#flag_ident);
          impl dyn #trait_ident {
              #[doc(hidden)]
              #flag_vis const fn cacheable_flag(name: &'static str, deserialize: DeserializeFn) -> #flag_ident {
                  #flag_ident { name, deserialize }
              }
          }

          use std::collections::BTreeMap;
          use std::collections::btree_map::Entry;
          static REGISTRY: once_cell::sync::Lazy<BTreeMap<&str, DeserializeFn>> = once_cell::sync::Lazy::new(|| {
              let mut map = BTreeMap::new();
              for flag in inventory::iter::<#flag_ident> {
                  let name = flag.name;
                  match map.entry(name) {
                      Entry::Vacant(val) => {
                          val.insert(flag.deserialize);
                      },
                      Entry::Occupied(_) => {
                          panic!("cacheable_dyn init global REGISTRY error, duplicate implementation of {name}");
                      }
                  }
              }
              map
          });
          impl AsDynConverter for Box<dyn #trait_ident> {
              type Context = #context;
              fn to_bytes(&self, context: &mut Self::Context) -> Result<Vec<u8>, SerializeError> {
                  let inner = self.as_ref();
                  let data = (String::from(inner.__cacheable_dyn_type_name()), inner.__cacheable_dyn_to_data(context)?);
                  rspack_cacheable::to_bytes(&data, context)
              }
              fn from_bytes(bytes: &[u8], context: &mut Self::Context) -> Result<Self, DeserializeError> where Self: Sized {
                  let (name, data) = rspack_cacheable::from_bytes::<(String, Vec<u8>), #context>(bytes, context)?;
                  let deserialize_fn = REGISTRY.get(name.as_str()).expect("unsupport data type when deserialize");
                  deserialize_fn(&data, context)
              }
          }
      };
  }
  .into()
}

pub fn impl_impl(args: CacheableDynArgs, mut input: ItemImpl) -> TokenStream {
  let context = &args.context;
  let trait_ident = &input.trait_.as_ref().unwrap().1;
  let target_ident = &input.self_ty;
  let target_ident_string = match &*input.self_ty {
    Type::Path(inner) => {
      let name = &inner.path.segments.last().unwrap().ident.to_string();
      quote! {#name}
    }
    _ => {
      panic!("cacheable_dyn unsupport this target")
    }
  };

  input.items.push(parse_quote! {
      #[doc(hidden)]
      fn __cacheable_dyn_type_name(&self) -> &'static str {
          #target_ident_string
      }
  });
  input.items.push(parse_quote! {
      #[doc(hidden)]
      fn __cacheable_dyn_to_data(&self, context: &mut #context) -> Result<Vec<u8>, rspack_cacheable::SerializeError> {
          rspack_cacheable::to_bytes(self, context)
      }
  });
  input.items.push(parse_quote! {
      #[doc(hidden)]
      fn __cacheable_dyn_from_data(bytes: &[u8], context: &mut #context) -> Result<Self, rspack_cacheable::DeserializeError> where Self: Sized {
          rspack_cacheable::from_bytes::<Self, #context>(bytes, context)
      }
  });

  quote! {
      #input

      #[allow(non_upper_case_globals)]
      const _: () = {
          use rspack_cacheable::__private::inventory;
          inventory::submit! {
              <dyn #trait_ident>::cacheable_flag(#target_ident_string, |bytes, context| {
                  Ok(Box::new(<#target_ident as #trait_ident>::__cacheable_dyn_from_data(bytes, context)?))
              })
          }
      };
  }
  .into()
}
