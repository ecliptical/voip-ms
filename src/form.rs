//! The `(name, value)` fields a request's parameters become on the wire.
//!
//! Both transports carry the same fields -- a GET on the query string, a
//! multipart POST as form parts -- so a parameter set is rendered once, here,
//! and the transport only decides where the fields ride. `reqwest`'s `.query()`
//! percent-encodes them into the query string; a form part carries the value
//! verbatim.
//!
//! The rendering matches `serde_urlencoded`, which is what `.query()` applies
//! to a parameter set handed to it directly. That agreement is what lets the
//! multipart form be built without first encoding the parameters into a URL and
//! reading them back out -- a round trip that allocated roughly four times an
//! upload's size to reach the same fields. This module's tests assert the two
//! renderings agree value by value, against `serde_urlencoded` itself.
//!
//! Rendering once for both transports puts the multipart POST's saving on the
//! GET's bill, so a field owns only what it must: see [`Field`]. What is left
//! is one `String` per parameter value, on a payload the 8190-byte request line
//! already bounds.
//!
//! **No field's wire form rests on a `Display` impl that could change.** A
//! wire form is a contract with VoIP.ms, where `Display` is free to render for
//! a person -- [`crate::Error`] prints `API status: did_in_use (DID Number is
//! already in use)` where [`crate::ApiStatus`] prints `did_in_use`.
//!
//! That rules out borrowing a *type's* rendering, not `to_string` as such. A
//! primitive integer's `Display` is specified by the standard library as its
//! decimal digits and cannot drift, so the integer arms use it. A float's is
//! equally fixed but fixed to something else -- `1` for `1.0`, and never an
//! exponent -- so the float arms render through `ryu`, which is what
//! `serde_urlencoded` and `serde_json` use for the same job. Nothing here calls
//! `to_string` on a domain type: one arrives through its own `Serialize` impl,
//! which hands this serializer a string or a number.

use std::borrow::Cow;
use std::fmt::{self, Display};

use serde::Serialize;
use serde::ser::{self, Impossible, Serializer};

/// One rendered field.
///
/// The name borrows where it can: a struct hands `serde` its field names as
/// `&'static str`, which is every generated parameter, so only a map key (a
/// caller's own `BTreeMap` or `json!`) has to be owned. The value never can be
/// -- `Serializer::serialize_str` elides its lifetime, so the text is only
/// guaranteed to live for that call.
pub(crate) type Field = (Cow<'static, str>, String);

/// Render `params` as the wire fields they carry.
///
/// A field whose value is absent (`None`) carries nothing at all, which is how
/// a query string omits it.
pub(crate) fn to_fields<P>(params: &P) -> Result<Vec<Field>, FormError>
where
    P: Serialize + ?Sized,
{
    let mut fields = Vec::new();
    params.serialize(FieldsSerializer {
        out: &mut fields,
        key: None,
    })?;

    Ok(fields)
}

/// Why a parameter set has no wire-field rendering.
#[derive(Debug)]
pub(crate) struct FormError(String);

impl FormError {
    /// The same message, naming the parameter it came from.
    fn in_field(self, field: &str) -> Self {
        Self(format!("parameter `{field}` {}", self.0))
    }

    /// The message, for the [`crate::ParamsError`] this becomes.
    pub(crate) fn into_message(self) -> String {
        self.0
    }
}

impl Display for FormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FormError {}

impl ser::Error for FormError {
    fn custom<T: Display>(msg: T) -> Self {
        Self(msg.to_string())
    }
}

/// Render one value and push it under `key`, unless it is absent.
fn push_field<T>(out: &mut Vec<Field>, key: Cow<'static, str>, value: &T) -> Result<(), FormError>
where
    T: ?Sized + Serialize,
{
    match value
        .serialize(PartSerializer)
        .map_err(|e| e.in_field(&key))?
    {
        Some(rendered) => {
            out.push((key, rendered));
            Ok(())
        }

        None => Ok(()),
    }
}

/// The parameter set itself: a struct, a map, or a sequence of pairs.
struct FieldsSerializer<'a> {
    out: &'a mut Vec<Field>,
    /// The map key awaiting its value. `serde` hands the two over separately.
    key: Option<String>,
}

/// The arms that are not a parameter set, each rejected the same way.
macro_rules! not_a_parameter_set {
    ($($method:ident($($arg:ident: $ty:ty),*);)*) => {
        $(
            fn $method(self $(, $arg: $ty)*) -> Result<Self::Ok, Self::Error> {
                $(let _ = $arg;)*
                Err(ser::Error::custom(
                    "parameters must be a struct, a map, or a sequence of name/value pairs",
                ))
            }
        )*
    };
}

impl<'a> Serializer for FieldsSerializer<'a> {
    type Ok = ();
    type Error = FormError;
    type SerializeSeq = PairsSerializer<'a>;
    type SerializeTuple = PairsSerializer<'a>;
    type SerializeTupleStruct = Impossible<(), FormError>;
    type SerializeTupleVariant = Impossible<(), FormError>;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<(), FormError>;

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(PairsSerializer { out: self.out })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(PairsSerializer { out: self.out })
    }

    /// A parameter set that is absent carries no fields, which is what an empty
    /// query string says.
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(ser::Error::custom(
            "parameters must be a struct, a map, or a sequence of name/value pairs",
        ))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(ser::Error::custom(
            "parameters must be a struct, a map, or a sequence of name/value pairs",
        ))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ser::Error::custom(
            "parameters must be a struct, a map, or a sequence of name/value pairs",
        ))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ser::Error::custom(
            "parameters must be a struct, a map, or a sequence of name/value pairs",
        ))
    }

    not_a_parameter_set! {
        serialize_bool(v: bool);
        serialize_i8(v: i8);
        serialize_i16(v: i16);
        serialize_i32(v: i32);
        serialize_i64(v: i64);
        serialize_i128(v: i128);
        serialize_u8(v: u8);
        serialize_u16(v: u16);
        serialize_u32(v: u32);
        serialize_u64(v: u64);
        serialize_u128(v: u128);
        serialize_f32(v: f32);
        serialize_f64(v: f64);
        serialize_char(v: char);
        serialize_str(v: &str);
        serialize_bytes(v: &[u8]);
    }

    /// A unit struct names no fields, so it is the empty parameter set -- the
    /// same answer as `None`, and what an empty query string says.
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom(
            "parameters must be a struct, a map, or a sequence of name/value pairs",
        ))
    }
}

impl ser::SerializeStruct for FieldsSerializer<'_> {
    type Ok = ();
    type Error = FormError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        push_field(self.out, Cow::Borrowed(key), value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeMap for FieldsSerializer<'_> {
    type Ok = ();
    type Error = FormError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.key = Some(
            key.serialize(PartSerializer)?
                .ok_or_else(|| ser::Error::custom("a parameter name cannot be absent"))?,
        );

        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| ser::Error::custom("a parameter value arrived before its name"))?;

        push_field(self.out, Cow::Owned(key), value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

/// A parameter set given as a sequence of `(name, value)` pairs.
struct PairsSerializer<'a> {
    out: &'a mut Vec<Field>,
}

impl ser::SerializeSeq for PairsSerializer<'_> {
    type Ok = ();
    type Error = FormError;

    fn serialize_element<T>(&mut self, pair: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        pair.serialize(PairSerializer { out: self.out })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeTuple for PairsSerializer<'_> {
    type Ok = ();
    type Error = FormError;

    fn serialize_element<T>(&mut self, pair: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        pair.serialize(PairSerializer { out: self.out })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

/// One element of a sequence of pairs, which must be a two-element tuple.
struct PairSerializer<'a> {
    out: &'a mut Vec<Field>,
}

/// The arms that are not a `(name, value)` pair, each rejected the same way.
macro_rules! not_a_pair {
    ($($method:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {
        $(
            fn $method(self $(, $arg: $ty)*) -> Result<$ret, Self::Error> {
                $(let _ = $arg;)*
                Err(ser::Error::custom(
                    "each element of a parameter sequence must be a name/value pair",
                ))
            }
        )*
    };
}

impl<'a> Serializer for PairSerializer<'a> {
    type Ok = ();
    type Error = FormError;
    type SerializeSeq = PairElements<'a>;
    type SerializeTuple = PairElements<'a>;
    type SerializeTupleStruct = PairElements<'a>;
    type SerializeTupleVariant = Impossible<(), FormError>;
    type SerializeMap = Impossible<(), FormError>;
    type SerializeStruct = Impossible<(), FormError>;
    type SerializeStructVariant = Impossible<(), FormError>;

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(PairElements {
            out: self.out,
            key: None,
        })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(PairElements {
            out: self.out,
            key: None,
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(PairElements {
            out: self.out,
            key: None,
        })
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(ser::Error::custom(
            "each element of a parameter sequence must be a name/value pair",
        ))
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(ser::Error::custom(
            "each element of a parameter sequence must be a name/value pair",
        ))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(ser::Error::custom(
            "each element of a parameter sequence must be a name/value pair",
        ))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ser::Error::custom(
            "each element of a parameter sequence must be a name/value pair",
        ))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ser::Error::custom(
            "each element of a parameter sequence must be a name/value pair",
        ))
    }

    not_a_pair! {
        serialize_bool(v: bool) -> ();
        serialize_i8(v: i8) -> ();
        serialize_i16(v: i16) -> ();
        serialize_i32(v: i32) -> ();
        serialize_i64(v: i64) -> ();
        serialize_i128(v: i128) -> ();
        serialize_u8(v: u8) -> ();
        serialize_u16(v: u16) -> ();
        serialize_u32(v: u32) -> ();
        serialize_u64(v: u64) -> ();
        serialize_u128(v: u128) -> ();
        serialize_f32(v: f32) -> ();
        serialize_f64(v: f64) -> ();
        serialize_char(v: char) -> ();
        serialize_str(v: &str) -> ();
        serialize_bytes(v: &[u8]) -> ();
        serialize_none() -> ();
        serialize_unit() -> ();
        serialize_unit_struct(name: &'static str) -> ();
        serialize_unit_variant(name: &'static str, index: u32, variant: &'static str) -> ();
    }
}

/// The two elements of one `(name, value)` pair, as `serde` hands them over.
struct PairElements<'a> {
    out: &'a mut Vec<Field>,
    key: Option<String>,
}

impl PairElements<'_> {
    fn take(&mut self, element: &(impl Serialize + ?Sized)) -> Result<(), FormError> {
        match self.key.take() {
            None => {
                self.key = Some(
                    element
                        .serialize(PartSerializer)?
                        .ok_or_else(|| ser::Error::custom("a parameter name cannot be absent"))?,
                );

                Ok(())
            }

            Some(key) => push_field(self.out, Cow::Owned(key), element),
        }
    }

    fn finish(self) -> Result<(), FormError> {
        match self.key {
            None => Ok(()),
            Some(_) => Err(ser::Error::custom(
                "a parameter pair must carry both a name and a value",
            )),
        }
    }
}

impl ser::SerializeSeq for PairElements<'_> {
    type Ok = ();
    type Error = FormError;

    fn serialize_element<T>(&mut self, element: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.take(element)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl ser::SerializeTuple for PairElements<'_> {
    type Ok = ();
    type Error = FormError;

    fn serialize_element<T>(&mut self, element: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.take(element)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl ser::SerializeTupleStruct for PairElements<'_> {
    type Ok = ();
    type Error = FormError;

    fn serialize_field<T>(&mut self, element: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.take(element)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

/// One parameter's value. `Ok(None)` is an absent value, which carries no field
/// at all.
///
/// Each arm renders a value whose text this crate controls: the module doc says
/// why that rules out a domain type's `Display` but not a primitive integer's.
struct PartSerializer;

/// An integer arm: its decimal digits, which is all `to_string` can produce for
/// a primitive integer.
macro_rules! integer_arms {
    ($($method:ident($ty:ty);)*) => {
        $(
            fn $method(self, v: $ty) -> Result<Self::Ok, Self::Error> {
                Ok(Some(v.to_string()))
            }
        )*
    };
}

/// The arms with no scalar form, each rejected the same way.
macro_rules! no_scalar_form {
    ($($method:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {
        $(
            fn $method(self $(, $arg: $ty)*) -> Result<$ret, Self::Error> {
                $(let _ = $arg;)*
                Err(ser::Error::custom("has no value a wire field can carry"))
            }
        )*
    };
}

impl Serializer for PartSerializer {
    type Ok = Option<String>;
    type Error = FormError;
    type SerializeSeq = Impossible<Option<String>, FormError>;
    type SerializeTuple = Impossible<Option<String>, FormError>;
    type SerializeTupleStruct = Impossible<Option<String>, FormError>;
    type SerializeTupleVariant = Impossible<Option<String>, FormError>;
    type SerializeMap = Impossible<Option<String>, FormError>;
    type SerializeStruct = Impossible<Option<String>, FormError>;
    type SerializeStructVariant = Impossible<Option<String>, FormError>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Some(if v { "true" } else { "false" }.to_owned()))
    }

    integer_arms! {
        serialize_i8(i8);
        serialize_i16(i16);
        serialize_i32(i32);
        serialize_i64(i64);
        serialize_i128(i128);
        serialize_u8(u8);
        serialize_u16(u16);
        serialize_u32(u32);
        serialize_u64(u64);
        serialize_u128(u128);
    }

    /// `ryu`'s rendering, which keeps the fraction on a whole float (`1.0`) and
    /// switches to an exponent for a large or small one (`1e300`).
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Some(ryu::Buffer::new().format(v).to_owned()))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Some(ryu::Buffer::new().format(v).to_owned()))
    }

    /// The character's own UTF-8, which is what a field carries.
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Some(String::from(v)))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Some(v.to_owned()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(None)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Some(variant.to_owned()))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(ser::Error::custom("has no value a wire field can carry"))
    }

    /// A byte string is its UTF-8 text, and an error when it is not UTF-8 --
    /// a field carries text, and there is no second encoding to fall back on.
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        match std::str::from_utf8(v) {
            Ok(text) => Ok(Some(text.to_owned())),
            Err(e) => Err(ser::Error::custom(format!("is not UTF-8: {e}"))),
        }
    }

    /// A unit struct carries its own name, which is the only thing it holds.
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Some(name.to_owned()))
    }

    no_scalar_form! {
        serialize_unit() -> Option<String>;
        serialize_seq(len: Option<usize>) -> Self::SerializeSeq;
        serialize_tuple(len: usize) -> Self::SerializeTuple;
        serialize_map(len: Option<usize>) -> Self::SerializeMap;
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(ser::Error::custom("has no value a wire field can carry"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ser::Error::custom("has no value a wire field can carry"))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(ser::Error::custom("has no value a wire field can carry"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ser::Error::custom("has no value a wire field can carry"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    /// The fields `reqwest`'s `.query()` would put on the query string, read
    /// back as pairs. Encoding and decoding are exact inverses, so what comes
    /// back is what rode.
    fn query_fields<T>(params: &T) -> Result<Vec<(String, String)>, ()>
    where
        T: ?Sized + Serialize,
    {
        let encoded = serde_urlencoded::to_string(params).map_err(|_| ())?;
        serde_urlencoded::from_str(&encoded).map_err(|_| ())
    }

    /// This module's rendering, with the borrowed names flattened so it can be
    /// compared against what the query string carries.
    fn fields_of<T>(params: &T) -> Result<Vec<(String, String)>, ()>
    where
        T: ?Sized + Serialize,
    {
        to_fields(params)
            .map(|fields| {
                fields
                    .into_iter()
                    .map(|(name, value)| (name.into_owned(), value))
                    .collect()
            })
            .map_err(|_| ())
    }

    /// Assert this module renders `params` into the same fields the query
    /// string carries -- including agreeing on a parameter set neither can
    /// render.
    #[track_caller]
    fn agrees<T>(params: &T)
    where
        T: ?Sized + Serialize,
    {
        assert_eq!(fields_of(params), query_fields(params));
    }

    /// The one field `params` renders to.
    #[track_caller]
    fn only_field<T: Serialize>(params: &T) -> (String, String) {
        let mut fields = fields_of(params).expect("renders");
        assert_eq!(fields.len(), 1, "expected exactly one field: {fields:?}");
        fields.pop().expect("one field")
    }

    /// One field of `value`, which is how a parameter reaches either transport.
    #[derive(Serialize)]
    struct One<T> {
        value: T,
    }

    fn one<T: Serialize>(value: T) -> One<T> {
        One { value }
    }

    #[derive(Serialize)]
    enum Choice {
        #[serde(rename = "ring-all")]
        RingAll,
    }

    #[derive(Serialize)]
    struct Wrapped(u32);

    #[derive(Serialize)]
    struct Unit;

    /// A field that reaches the serializer through `serialize_bytes`, as
    /// `serde_bytes` and any hand-written impl that calls it do.
    struct Bytes(&'static [u8]);

    impl Serialize for Bytes {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_bytes(self.0)
        }
    }

    #[test]
    fn scalars_render_as_the_query_string_renders_them() {
        agrees(&one(true));
        agrees(&one(false));
        agrees(&one("plain"));
        // The characters a query string escapes: they must survive as
        // themselves, since a form part carries them unescaped.
        agrees(&one("a+b&c=d e%f"));
        agrees(&one('x'));
        agrees(&one(-5i64));
        agrees(&one(u64::MAX));
        agrees(&one(i128::MIN));
        agrees(&one(0.5f64));
        agrees(&one(1.0f32));
        agrees(&one(Choice::RingAll));
        agrees(&one(Wrapped(7)));
        agrees(&one(Some(3u32)));
    }

    /// Where rendering through `Display` would have reached the wire with a
    /// different value. The rule this module states is broader -- no arm goes
    /// through `Display` -- but the integer arms agree with it by coincidence,
    /// so these are the cases that can catch a regression.
    #[test]
    fn a_float_is_not_rendered_the_way_a_reader_would_see_it() {
        for (value, wire) in [(1.0f64, "1.0"), (1e300, "1e300"), (1.0e-7, "1e-7")] {
            assert_eq!(
                only_field(&one(value)),
                ("value".to_string(), wire.to_string())
            );
            assert_ne!(
                value.to_string(),
                wire,
                "the premise: `Display` writes something else"
            );
        }
    }

    /// The integer arms do use `to_string`, which is sound because a primitive
    /// integer's `Display` is its decimal digits by specification. Pinned at the
    /// range ends, where a hand-rolled encoder would be the thing that slipped.
    #[test]
    fn an_integer_renders_as_its_digits() {
        agrees(&one(i64::MIN));
        agrees(&one(u128::MAX));
        assert_eq!(
            only_field(&one(i8::MIN)),
            ("value".to_string(), "-128".to_string())
        );
    }

    #[test]
    fn an_absent_value_carries_no_field_at_all() {
        agrees(&one(Option::<u32>::None));
        assert!(to_fields(&one(Option::<u32>::None)).unwrap().is_empty());
    }

    /// What [`Field`]'s `Cow` is for: the common path copies values only.
    #[test]
    fn a_struct_field_name_is_borrowed_rather_than_copied() {
        let from_struct = to_fields(&one("x")).unwrap();
        assert!(
            matches!(from_struct[0].0, Cow::Borrowed(_)),
            "a generated parameter's name is a &'static str serde hands over"
        );

        let from_map = to_fields(&BTreeMap::from([("value", "x")])).unwrap();
        assert!(
            matches!(from_map[0].0, Cow::Owned(_)),
            "a map key is rendered, so it is the one name that has to be owned"
        );
    }

    #[test]
    fn a_parameter_set_can_be_a_struct_a_map_or_a_sequence_of_pairs() {
        agrees(&BTreeMap::from([("b", "2"), ("a", "1")]));
        agrees(&[("a", "1"), ("b", "2")][..]);
        agrees(&vec![("a".to_string(), "1".to_string())]);
        agrees(&serde_json::json!({ "a": "1", "b": 2 }));
        // No parameters at all is an empty field list, not a failure.
        agrees(&());
        assert!(to_fields(&()).unwrap().is_empty());
    }

    /// The arms a caller's own params type can reach that no generated struct
    /// does. Each one used to ride the query string before the fields were
    /// rendered here, so each has to keep riding it.
    #[test]
    fn the_arms_only_a_hand_written_params_type_reaches() {
        // Bytes carry their text; invalid UTF-8 has no field form, and neither
        // encoder invents one.
        agrees(&one(Bytes(b"ab cd")));
        agrees(&one(Bytes(b"\xff\xfe")));
        assert!(to_fields(&one(Bytes(b"\xff\xfe"))).is_err());

        // A unit struct is its name as a value, and no fields at all as the
        // parameter set.
        agrees(&one(Unit));
        agrees(&Unit);
        assert_eq!(
            only_field(&one(Unit)),
            ("value".to_string(), "Unit".to_string())
        );
        assert!(to_fields(&Unit).unwrap().is_empty());
    }

    #[test]
    fn a_value_with_no_scalar_form_is_refused_by_name() {
        let nested = serde_json::json!({ "routing": { "kind": "sys" } });
        let error = to_fields(&nested).unwrap_err().into_message();
        assert!(error.contains("`routing`"), "{error}");
        assert!(error.contains("wire field"), "{error}");
        // The query string refuses it too, so neither transport is the one
        // that could have carried it.
        assert!(query_fields(&nested).is_err());

        agrees(&one(vec![1u32, 2]));
        agrees(&one(()));
    }

    #[test]
    fn a_parameter_set_that_is_not_a_set_of_fields_is_refused() {
        let error = to_fields("just a string").unwrap_err().into_message();
        assert!(error.contains("struct, a map, or a sequence"), "{error}");
        assert!(to_fields(&42u32).is_err());
        assert!(to_fields(&serde_json::json!([1, 2, 3])).is_err());
    }

    #[test]
    fn the_generated_parameters_render_the_same_on_both_transports() {
        use crate::*;

        // Every scalar kind the generated surface carries: a string, an id, a
        // decimal, a flag with a wire spelling of its own, a domain type with a
        // custom `Display`, and a named zone.
        agrees(&SetForwardingParams {
            forwarding: Some(19183),
            phone_number: Some("15555550100".into()),
            description: Some("desk & phone = ok".into()),
            pause: Some("1.5".parse().unwrap()),
            diversion_header: Some(true),
            ..Default::default()
        });
        agrees(&SetTimeConditionParams {
            name: Some("after hours".into()),
            routing_match: Some(Routing::System("hangup".into())),
            ..Default::default()
        });
        agrees(&GetCDRParams {
            date_from: Some(chrono::NaiveDate::from_ymd_opt(2026, 9, 16).unwrap()),
            timezone: Some(chrono_tz::Tz::Asia__Kolkata),
            ..Default::default()
        });
        agrees(&SetRecordingParams {
            name: Some("greeting".into()),
            file: Some("UklGRiQAAABXQVZF".into()),
            ..Default::default()
        });
    }
}
