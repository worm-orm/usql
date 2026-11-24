//! The binary format of `NUMERIC`: groups of four decimal digits.

use alloc::{format, string::String, vec, vec::Vec};
use bytes::{BufMut, BytesMut};
use core::fmt::Write;

use super::BoxError;

const NEGATIVE: u16 = 0x4000;
const NAN: u16 = 0xC000;
const INFINITY: u16 = 0xD000;
const NEG_INFINITY: u16 = 0xF000;

const MAX_SCALE: usize = 16383;
const MAX_INTEGER: usize = 131072;

pub fn encode(input: &str, out: &mut BytesMut) -> Result<(), BoxError> {
    let input = input.trim();
    let special = match input {
        "NaN" => Some(NAN),
        "inf" | "Infinity" => Some(INFINITY),
        "-inf" | "-Infinity" => Some(NEG_INFINITY),
        _ => None,
    };
    if let Some(sign) = special {
        write(out, &[], 0, sign, 0);
        return Ok(());
    }

    let invalid = || -> BoxError { format!("invalid numeric: {input:?}").into() };

    let (negative, rest) = match input.as_bytes().first() {
        Some(b'-') => (true, &input[1..]),
        Some(b'+') => (false, &input[1..]),
        _ => (false, input),
    };
    let (mantissa, exponent) = match rest.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().map_err(|_| invalid())?),
        None => (rest, 0),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let is_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if !is_digits(integer) || !is_digits(fraction) || integer.len() + fraction.len() == 0 {
        return Err(invalid());
    }

    let mut digits: Vec<u8> = integer
        .bytes()
        .chain(fraction.bytes())
        .map(|b| b - b'0')
        .collect();

    // Position of the decimal point in `digits`
    let point = integer.len() as i64 + i64::from(exponent);
    let scale = (digits.len() as i64 - point).max(0);
    if scale > MAX_SCALE as i64 || point > MAX_INTEGER as i64 {
        return Err(format!("numeric is out of range: {input:?}").into());
    }
    if point < 0 {
        digits.splice(0..0, vec![0; -point as usize]);
    }
    let point = point.max(0) as usize;
    if digits.len() < point {
        digits.resize(point, 0);
    }

    // Align the groups to the decimal point
    let pad = (4 - point % 4) % 4;
    digits.splice(0..0, vec![0; pad]);
    let point = point + pad;
    digits.resize(digits.len().next_multiple_of(4), 0);

    let mut groups: Vec<u16> = digits
        .chunks(4)
        .map(|c| c.iter().fold(0, |n, d| n * 10 + u16::from(*d)))
        .collect();
    let mut weight = (point / 4) as i32 - 1;

    while groups.last() == Some(&0) {
        groups.pop();
    }
    let leading = groups.iter().take_while(|g| **g == 0).count();
    groups.drain(..leading);
    weight -= leading as i32;
    if groups.is_empty() {
        weight = 0;
    }

    let sign = if negative && !groups.is_empty() {
        NEGATIVE
    } else {
        0
    };
    write(out, &groups, weight as i16, sign, scale as u16);

    Ok(())
}

fn write(out: &mut BytesMut, groups: &[u16], weight: i16, sign: u16, scale: u16) {
    out.put_u16(groups.len() as u16);
    out.put_i16(weight);
    out.put_u16(sign);
    out.put_u16(scale);
    for group in groups {
        out.put_u16(*group);
    }
}

pub fn decode(raw: &[u8]) -> Result<String, BoxError> {
    let (words, rest) = raw.as_chunks::<2>();
    let words: Vec<u16> = words.iter().map(|w| u16::from_be_bytes(*w)).collect();
    let [len, weight, sign, scale, groups @ ..] = words.as_slice() else {
        return Err("invalid numeric".into());
    };
    if !rest.is_empty() || groups.len() != usize::from(*len) || groups.iter().any(|g| *g > 9999) {
        return Err("invalid numeric".into());
    }

    let mut out = String::new();
    match *sign {
        0 => {}
        NEGATIVE => out.push('-'),
        NAN => return Ok("NaN".into()),
        INFINITY => return Ok("Infinity".into()),
        NEG_INFINITY => return Ok("-Infinity".into()),
        _ => return Err("invalid numeric".into()),
    }

    let weight = i32::from(*weight as i16);
    let group = |exponent: i32| -> u16 {
        usize::try_from(weight - exponent)
            .ok()
            .and_then(|i| groups.get(i).copied())
            .unwrap_or(0)
    };

    if weight < 0 {
        out.push('0');
    }
    for exponent in (0..=weight).rev() {
        if exponent == weight {
            write!(out, "{}", group(exponent))?;
        } else {
            write!(out, "{:04}", group(exponent))?;
        }
    }

    let scale = usize::from(*scale);
    if scale > 0 {
        out.push('.');
        let end = out.len() + scale;
        let mut exponent = -1;
        while out.len() < end {
            write!(out, "{:04}", group(exponent))?;
            exponent -= 1;
        }
        out.truncate(end);
    }

    Ok(out)
}
