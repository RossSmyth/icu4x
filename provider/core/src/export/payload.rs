// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use core::any::Any;

use crate::dynutil::UpcastDataPayload;
use crate::prelude::*;
use alloc::sync::Arc;
use databake::{Bake, BakeSize, CrateEnv, TokenStream};
use yoke::trait_hack::YokeTraitHack;
use yoke::*;

trait ExportableDataPayload {
    fn bake_yoke(&self, env: &CrateEnv) -> TokenStream;
    fn bake_size(&self) -> usize;
    fn serialize_yoke(
        &self,
        serializer: &mut dyn erased_serde::Serializer,
    ) -> Result<(), DataError>;
    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn ExportableDataPayload) -> bool;
}

impl<M: DynamicDataMarker> ExportableDataPayload for DataPayload<M>
where
    for<'a> <M::DataStruct as Yokeable<'a>>::Output: Bake + BakeSize + serde::Serialize,
    for<'a> YokeTraitHack<<M::DataStruct as Yokeable<'a>>::Output>: PartialEq,
{
    fn bake_yoke(&self, ctx: &CrateEnv) -> TokenStream {
        self.get().bake(ctx)
    }

    fn bake_size(&self) -> usize {
        core::mem::size_of::<<M::DataStruct as Yokeable>::Output>() + self.get().borrows_size()
    }

    fn serialize_yoke(
        &self,
        serializer: &mut dyn erased_serde::Serializer,
    ) -> Result<(), DataError> {
        use erased_serde::Serialize;
        self.get()
            .erased_serialize(serializer)
            .map_err(|e| DataError::custom("Serde export").with_display_context(&e))?;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_dyn(&self, other: &dyn ExportableDataPayload) -> bool {
        match other.as_any().downcast_ref::<Self>() {
            Some(downcasted) => (*self).eq(downcasted),
            None => {
                debug_assert!(
                    false,
                    "cannot compare ExportableDataPayloads of different types: self is {:?} but other is {:?}",
                    self.type_id(),
                    other.as_any().type_id(),
                );
                false
            }
        }
    }
}

#[doc(hidden)] // macro
#[derive(yoke::Yokeable, Clone)]
pub struct ExportBox {
    payload: Arc<dyn ExportableDataPayload + Sync + Send>,
}

impl PartialEq for ExportBox {
    fn eq(&self, other: &Self) -> bool {
        self.payload.eq_dyn(&*other.payload)
    }
}

impl Eq for ExportBox {}

impl core::fmt::Debug for ExportBox {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ExportBox")
            .field("payload", &"<payload>")
            .finish()
    }
}

impl<M> UpcastDataPayload<M> for ExportMarker
where
    M: DynamicDataMarker,
    M::DataStruct: Sync + Send,
    for<'a> <M::DataStruct as Yokeable<'a>>::Output: Bake + BakeSize + serde::Serialize,
    for<'a> YokeTraitHack<<M::DataStruct as Yokeable<'a>>::Output>: PartialEq,
{
    fn upcast(other: DataPayload<M>) -> DataPayload<ExportMarker> {
        DataPayload::from_owned(ExportBox {
            payload: Arc::new(other),
        })
    }
}

impl DataPayload<ExportMarker> {
    /// Serializes this [`DataPayload`] into a serializer using Serde.
    ///
    /// # Examples
    ///
    /// ```
    /// use icu_provider::export::*;
    /// use icu_provider::dynutil::UpcastDataPayload;
    /// use icu_provider::hello_world::HelloWorldV1Marker;
    /// use icu_provider::prelude::*;
    ///
    /// // Create an example DataPayload
    /// let payload: DataPayload<HelloWorldV1Marker> = Default::default();
    /// let export: DataPayload<ExportMarker> = UpcastDataPayload::upcast(payload);
    ///
    /// // Serialize the payload to a JSON string
    /// let mut buffer: Vec<u8> = vec![];
    /// export
    ///     .serialize(&mut serde_json::Serializer::new(&mut buffer))
    ///     .expect("Serialization should succeed");
    /// assert_eq!(r#"{"message":"(und) Hello World"}"#.as_bytes(), buffer);
    /// ```
    pub fn serialize<S>(&self, serializer: S) -> Result<(), DataError>
    where
        S: serde::Serializer,
        S::Ok: 'static, // erased_serde requirement, cannot return values in `Ok`
    {
        self.get()
            .payload
            .serialize_yoke(&mut <dyn erased_serde::Serializer>::erase(serializer))
    }

    /// Serializes this [`DataPayload`]'s value into a [`TokenStream`]
    /// using its [`Bake`] implementations.
    ///
    /// # Examples
    ///
    /// ```
    /// use icu_provider::export::*;
    /// use icu_provider::dynutil::UpcastDataPayload;
    /// use icu_provider::hello_world::HelloWorldV1Marker;
    /// use icu_provider::prelude::*;
    /// # use databake::quote;
    /// # use std::collections::BTreeSet;
    ///
    /// // Create an example DataPayload
    /// let payload: DataPayload<HelloWorldV1Marker> = Default::default();
    /// let export: DataPayload<ExportMarker> = UpcastDataPayload::upcast(payload);
    ///
    /// let env = databake::CrateEnv::default();
    /// let tokens = export.tokenize(&env);
    /// assert_eq!(
    ///     quote! {
    ///         icu_provider::hello_world::HelloWorldV1 {
    ///             message: alloc::borrow::Cow::Borrowed("(und) Hello World"),
    ///         }
    ///     }
    ///     .to_string(),
    ///     tokens.to_string()
    /// );
    /// assert_eq!(
    ///     env.into_iter().collect::<BTreeSet<_>>(),
    ///     ["icu_provider", "alloc"]
    ///         .into_iter()
    ///         .collect::<BTreeSet<_>>()
    /// );
    /// ```
    pub fn tokenize(&self, env: &CrateEnv) -> TokenStream {
        self.get().payload.bake_yoke(env)
    }

    /// Returns the data size using postcard encoding
    pub fn postcard_size(&self) -> usize {
        use postcard::ser_flavors::{Flavor, Size};
        let mut serializer = postcard::Serializer {
            output: Size::default(),
        };
        let _infallible = self
            .get()
            .payload
            .serialize_yoke(&mut <dyn erased_serde::Serializer>::erase(&mut serializer));

        serializer.output.finalize().unwrap_or_default()
    }

    /// Returns an estimate of the baked size, made up of the size of the struct itself,
    /// as well as the sizes of all its static borrows.
    ///
    /// As static borrows are deduplicated by the linker, this is often overcounting.
    pub fn baked_size(&self) -> usize {
        self.get().payload.bake_size()
    }
}

impl core::hash::Hash for DataPayload<ExportMarker> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        use postcard::ser_flavors::Flavor;

        struct HashFlavor<'a, H>(&'a mut H);
        impl<'a, H: core::hash::Hasher> Flavor for HashFlavor<'a, H> {
            type Output = ();

            fn try_push(&mut self, data: u8) -> postcard::Result<()> {
                self.0.write_u8(data);
                Ok(())
            }

            fn finalize(self) -> postcard::Result<Self::Output> {
                Ok(())
            }
        }

        let _infallible =
            self.get()
                .payload
                .serialize_yoke(&mut <dyn erased_serde::Serializer>::erase(
                    &mut postcard::Serializer {
                        output: HashFlavor(state),
                    },
                ));
    }
}

/// Marker type for [`ExportBox`].
#[allow(clippy::exhaustive_structs)] // marker type
#[derive(Debug)]
pub struct ExportMarker {}

impl DynamicDataMarker for ExportMarker {
    type DataStruct = ExportBox;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hello_world::*;

    #[test]
    fn test_compare_with_dyn() {
        let payload1: DataPayload<HelloWorldV1Marker> = DataPayload::from_owned(HelloWorldV1 {
            message: "abc".into(),
        });
        let payload2: DataPayload<HelloWorldV1Marker> = DataPayload::from_owned(HelloWorldV1 {
            message: "abc".into(),
        });
        let payload3: DataPayload<HelloWorldV1Marker> = DataPayload::from_owned(HelloWorldV1 {
            message: "def".into(),
        });

        assert!(payload1.eq_dyn(&payload2));
        assert!(payload2.eq_dyn(&payload1));

        assert!(!payload1.eq_dyn(&payload3));
        assert!(!payload3.eq_dyn(&payload1));
    }

    #[test]
    fn test_export_marker_partial_eq() {
        let payload1: DataPayload<ExportMarker> =
            UpcastDataPayload::upcast(DataPayload::<HelloWorldV1Marker>::from_owned(
                HelloWorldV1 {
                    message: "abc".into(),
                },
            ));
        let payload2: DataPayload<ExportMarker> =
            UpcastDataPayload::upcast(DataPayload::<HelloWorldV1Marker>::from_owned(
                HelloWorldV1 {
                    message: "abc".into(),
                },
            ));
        let payload3: DataPayload<ExportMarker> =
            UpcastDataPayload::upcast(DataPayload::<HelloWorldV1Marker>::from_owned(
                HelloWorldV1 {
                    message: "def".into(),
                },
            ));

        assert_eq!(payload1, payload2);
        assert_eq!(payload2, payload1);
        assert_ne!(payload1, payload3);
        assert_ne!(payload3, payload1);
    }
}
