mod arg;
mod result;
#[cfg(feature = "tauri")]
mod tauri;

#[cfg(feature = "tauri")]
pub use self::tauri::*;
pub use arg::*;
pub use result::*;

use crate::*;

/// Returns a [`FunctionDataType`] for a given function that has been annotated with
/// [`specta`](macro@crate::specta).
///
/// # Examples
///
/// ```rust
/// use specta::*;
///
/// #[specta]
/// fn some_function(name: String, age: i32) -> bool {
///     true
/// }
///
/// fn main() {
///     let typ = fn_datatype!(some_function).unwrap();
///
///     assert_eq!(typ.name, "some_function");
///     assert_eq!(typ.args.len(), 2);
///     assert_eq!(typ.result, DataType::Primitive(PrimitiveType::bool))
/// }
/// ```
#[macro_export]
macro_rules! fn_datatype {
    ($function:path) => {{
        let mut type_map = $crate::TypeDefs::default();

        $crate::fn_datatype!(type_map, $function)
    }};
    ($type_map:ident, $function:path) => {{
        let type_map: &mut $crate::TypeDefs = &mut $type_map;

        $crate::internal::fn_datatype!(type_map, $function)
    }};
}

/// Contains type information about a function annotated with [`specta`](macro@crate::specta).
/// Returned by [`fn_datatype`].
#[derive(Debug, Clone)]
pub struct FunctionDataType {
    /// Whether the function is async.
    pub asyncness: bool,
    /// The function's name.
    pub name: &'static str,
    /// The name and type of each of the function's arguments.
    pub args: Vec<(&'static str, DataType)>,
    /// The return type of the function.
    pub result: DataType,
    /// The function's documentation. Detects both `///` and `#[doc = ...]` style documentation.
    pub docs: Vec<&'static str>,
}

/// Implemented by functions that can be annoatated with [`specta`](crate::specta).
pub trait SpectaFunction<TMarker> {
    /// Gets the type of a function as a [`FunctionDataType`].
    fn to_datatype(
        asyncness: bool,
        name: &'static str,
        type_map: &mut TypeDefs,
        fields: &[&'static str],
        docs: Vec<&'static str>,
    ) -> Result<FunctionDataType, ExportError>;
}

impl<TResultMarker, TResult: SpectaFunctionResult<TResultMarker>> SpectaFunction<TResultMarker>
    for fn() -> TResult
{
    fn to_datatype(
        asyncness: bool,
        name: &'static str,
        type_map: &mut TypeDefs,
        _fields: &[&'static str],
        docs: Vec<&'static str>,
    ) -> Result<FunctionDataType, ExportError> {
        TResult::to_datatype(DefOpts {
            parent_inline: false,
            type_map,
        })
        .map(|result| FunctionDataType {
            asyncness,
            name,
            args: vec![],
            result,
            docs,
        })
    }
}

#[doc(hidden)]
/// A helper for exporting a command to a [`CommandDataType`].
/// You shouldn't use this directly and instead should use [`fn_datatype!`](crate::fn_datatype).
pub fn get_datatype_internal<TMarker, T: SpectaFunction<TMarker>>(
    _: T,
    asyncness: bool,
    name: &'static str,
    type_map: &mut TypeDefs,
    fields: &[&'static str],
    docs: Vec<&'static str>,
) -> Result<FunctionDataType, ExportError> {
    T::to_datatype(asyncness, name, type_map, fields, docs)
}

macro_rules! impl_typed_command {
    ( impl $($i:ident),* ) => {
       paste::paste! {
            impl<
                TResultMarker,
                TResult: SpectaFunctionResult<TResultMarker>,
                $([<$i Marker>]),*,
                $($i: SpectaFunctionArg<[<$i Marker>]>),*
            > SpectaFunction<(TResultMarker, $([<$i Marker>]),*)> for fn($($i),*) -> TResult {
                fn to_datatype(
                    asyncness: bool,
                    name: &'static str,
                    type_map: &mut TypeDefs,
                    fields: &[&'static str],
                    docs: Vec<&'static str>,
                ) -> Result<FunctionDataType, ExportError> {
                    let mut fields = fields.into_iter();

                    Ok(FunctionDataType {
                        asyncness,
                        name,
                        docs,
                        args: [$(
                            fields
                                .next()
                                .map_or_else(
                                    || Ok(None),
                                    |field| $i::to_datatype(DefOpts {
                                        parent_inline: false,
                                        type_map,
                                    }).map(|v| v.map(|ty| (*field, ty)))
                                )?
                        ),*,]
                        .into_iter()
                        .filter_map(|v| v)
                        .collect(),
                        result: TResult::to_datatype(DefOpts {
                            parent_inline: false,
                            type_map,
                        })?,
                    })
                }
            }
        }
    };
    ( $i2:ident $(, $i:ident)* ) => {
        impl_typed_command!(impl $i2 $(, $i)* );
        impl_typed_command!($($i),*);
    };
    () => {};
}

impl_typed_command!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);

/// Collects function types into a [`Vec`],
/// and all downstream types into a [`TypeDefs`] instance.
///
/// Specifying a `type_map` argument allows a custom [`TypeDefs`] to be used.
///
/// # Examples
///
/// ```rust
/// use specta::*;
///
/// #[specta]
/// fn some_function(name: String, age: i32) -> bool {
///     true
/// }
///
/// fn main() {
///     // `type_defs` is created internally
///     let (functions, type_defs) = functions::collect_types![some_function].unwrap();
///
///     let custom_type_defs = TypeDefs::default();
///
///     // `type_defs` is provided.
///     // This can be used when integrating multiple specta-enabled libraries.
///     let (functions, custom_type_defs) = functions::collect_types![
///         type_map: custom_type_defs,
///         some_function
///     ].unwrap();
/// }
/// ````
#[macro_export]
macro_rules! collect_types {
    (type_map: $type_map:ident, $($command:path),* $(,)?) => {{
        let mut type_map: $crate::TypeDefs = $type_map;

        {
            fn export(mut type_map: $crate::TypeDefs) -> ::std::result::Result<(Vec<$crate::functions::FunctionDataType>, $crate::TypeDefs), $crate::ExportError> {
                Ok((
                    vec![
                        $($crate::fn_datatype!(type_map, $command)?),*
                    ],
                    type_map,
                ))
            }

            export(type_map)
        }
    }};
    ($($command:path),* $(,)?) => {{
        let mut type_map = $crate::TypeDefs::default();
        $crate::functions::collect_types!(type_map: type_map, $($command),*)
    }};
}

pub use collect_types;
