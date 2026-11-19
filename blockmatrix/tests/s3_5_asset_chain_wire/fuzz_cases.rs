use super::*;

pub(super) fn every_truncation(good: &[u8], note: &mut impl FnMut(bool)) {
    // (1) Every truncation; only the full message may decode.
    let mut truncations_decoded = 0usize;
    for cut in 0..=good.len() {
        let ok = decode_presented_asset_chain(&good[..cut]).is_ok();
        if ok {
            truncations_decoded += 1;
            assert_eq!(
                cut,
                good.len(),
                "a truncated message decoded at {cut} bytes"
            );
        }
        note(ok);
    }
    assert_eq!(truncations_decoded, 1);
}

pub(super) fn every_tag(good: &[u8], note: &mut impl FnMut(bool)) {
    // (2) Every possible tag byte on an otherwise valid message.
    for tag in 0..=u8::MAX {
        let mut framed = good.to_vec();
        framed[0] = tag;
        let out = decode_presented_asset_chain(&framed);
        if tag == ASSET_CHAIN_TAG {
            assert!(out.is_ok(), "the real tag must still decode");
        } else {
            assert_eq!(out, Err(AssetChainWireError::WrongTag { got: tag }));
        }
        note(out.is_ok());
    }
}

pub(super) fn lying_lengths(good: &[u8], note: &mut impl FnMut(bool)) {
    // (3) Lying lengths: extremes, cap boundary, every power of two.
    let mut lengths: Vec<u32> = vec![
        0,
        1,
        u32::MAX,
        u32::MAX - 1,
        MAX_ASSET_CHAIN_WIRE_BYTES as u32,
        (MAX_ASSET_CHAIN_WIRE_BYTES as u32).wrapping_add(1),
        (MAX_ASSET_CHAIN_WIRE_BYTES as u32).wrapping_sub(1),
        good.len() as u32,
        (good.len() as u32).wrapping_sub(5),
        i32::MAX as u32,
    ];
    lengths.extend((0..32).map(|shift| 1u32 << shift));
    for declared in lengths {
        let mut framed = good.to_vec();
        framed[1..5].copy_from_slice(&declared.to_le_bytes());
        note(decode_presented_asset_chain(&framed).is_ok());
        for body in [0usize, 1, 4, 37] {
            let mut short = vec![ASSET_CHAIN_TAG];
            short.extend_from_slice(&declared.to_le_bytes());
            short.extend(std::iter::repeat(0xABu8).take(body));
            note(decode_presented_asset_chain(&short).is_ok());
        }
    }
}

pub(super) fn single_bit_flips(good: &[u8], prng: &mut Prng, note: &mut impl FnMut(bool)) {
    // (4) Single-bit flips across the whole valid message.
    for _ in 0..20_000 {
        let mut framed = good.to_vec();
        let at = prng.below(framed.len());
        framed[at] ^= 1u8 << (prng.byte() % 8);
        note(decode_presented_asset_chain(&framed).is_ok());
    }
}

pub(super) fn random_payloads(random_cases: usize, prng: &mut Prng, note: &mut impl FnMut(bool)) {
    // (5) Bulk random payloads, framed and unframed.
    for _ in 0..random_cases {
        let len = prng.below(96);
        let mut noise: Vec<u8> = (0..len).map(|_| prng.byte()).collect();
        note(decode_presented_asset_chain(&noise).is_ok());

        let body_len = prng.below(64);
        noise.clear();
        noise.push(ASSET_CHAIN_TAG);
        noise.extend_from_slice(&(body_len as u32).to_le_bytes());
        noise.extend((0..body_len).map(|_| prng.byte()));
        note(decode_presented_asset_chain(&noise).is_ok());
    }
}
