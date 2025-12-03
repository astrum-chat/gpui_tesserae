use gpui::{AbsoluteLength, DefiniteLength, Pixels, SharedString, px, rems};
use serde::{Deserialize, Deserializer, de::Error};
use smallvec::SmallVec;

use crate::ThemeVariant;

pub fn de_string_or_non_empty_list<'de, D>(
    deserializer: D,
) -> Result<SmallVec<[SharedString; 1]>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        One(SharedString),
        Many(SmallVec<[SharedString; 1]>),
    }

    let value = StringOrVec::deserialize(deserializer)?;

    match value {
        StringOrVec::One(string) => Ok(SmallVec::from_buf([string])),
        StringOrVec::Many(vec) => {
            if vec.len() == 0 {
                return Err(D::Error::custom("list can't be empty."));
            }

            Ok(vec)
        }
    }
}

pub fn de_variants<'de, D>(deserializer: D) -> Result<SmallVec<[ThemeVariant; 2]>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = SmallVec::deserialize(deserializer)?;

    if value.len() == 0 {
        return Err(D::Error::custom(
            "at least one theme variant needs to be provided.",
        ));
    }

    Ok(value)
}

pub fn de_pixels<'de, D>(deserializer: D) -> Result<Pixels, D::Error>
where
    D: Deserializer<'de>,
{
    match StringOrFloat::deserialize(deserializer)? {
        StringOrFloat::String(string) => {
            let string = match string.strip_suffix("px") {
                Some(string) => string,
                None => return Err(D::Error::custom("expected string to end with 'px'")),
            };

            match string.parse::<f32>() {
                Ok(pixels) => Ok(px(pixels)),
                Err(_) => Err(D::Error::custom("could not convert string into pixels")),
            }
        }

        StringOrFloat::Float(pixels) => Ok(px(pixels)),
    }
}

pub fn de_abs_length<'de, D>(deserializer: D) -> Result<AbsoluteLength, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match StringOrFloat::deserialize(deserializer)? {
        StringOrFloat::Float(num) => return Ok(AbsoluteLength::Pixels(px(num))),

        StringOrFloat::String(string) => {
            if let Some(string) = string.strip_suffix("rem")
                && let Ok(value) = string.parse::<f32>()
            {
                return Ok(AbsoluteLength::Rems(rems(value)));
            } else if let Some(string) = string.strip_suffix("px")
                && let Ok(value) = string.parse::<f32>()
            {
                return Ok(AbsoluteLength::Pixels(px(value)));
            }
        }
    }

    Err(serde::de::Error::custom(
        "expected f32 or string containing a f32 ending with 'rem' or 'px'",
    ))
}

pub fn de_def_length<'de, D>(deserializer: D) -> Result<DefiniteLength, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match StringOrFloat::deserialize(deserializer)? {
        StringOrFloat::Float(num) => {
            return Ok(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(
                num as f32
            ))));
        }

        StringOrFloat::String(string) => {
            if let Some(string) = string.strip_suffix("%")
                && let Ok(value) = string.parse::<f32>()
            {
                return Ok(DefiniteLength::Fraction(value / 100.));
            }

            if let Some(string) = string.strip_suffix("rem")
                && let Ok(value) = string.parse::<f32>()
            {
                return Ok(DefiniteLength::Absolute(AbsoluteLength::Rems(rems(value))));
            } else if let Some(string) = string.strip_suffix("px")
                && let Ok(value) = string.parse::<f32>()
            {
                return Ok(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(value))));
            }
        }
    }

    Err(serde::de::Error::custom(
        "expected f32 or string containing a f32 ending with 'rem' or 'px'",
    ))
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrFloat {
    String(String),
    Float(f32),
}
