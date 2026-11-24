//! Pure-Rust, allocation-free FIPS 180-4 SHA-256 and grid integrity checks.
//!
//! Avoids bringing external cryptographic crates into `no_std` tier-0 models
//! while letting [`super::AtmosphericGridProvider`] implementations enforce
//! exact SHA-256 verification over caller-supplied ITU atmospheric grids.

use crate::error::PropagationError;

use super::AtmosphericGridProvider;

const INITIAL_STATE: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

const ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// Streaming FIPS 180-4 SHA-256 hasher (`no_std`, zero allocation).
#[derive(Clone, Debug)]
pub struct Sha256 {
    state: [u32; 8],
    bit_len: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// Initializes a fresh SHA-256 context.
    pub const fn new() -> Self {
        Self {
            state: INITIAL_STATE,
            bit_len: 0,
            buffer: [0_u8; 64],
            buffer_len: 0,
        }
    }

    /// Feeds `data` into the digest.
    pub fn update(&mut self, data: &[u8]) {
        self.bit_len = self
            .bit_len
            .wrapping_add((data.len() as u64).wrapping_mul(8));
        let mut rest = data;
        if self.buffer_len > 0 {
            let take = (64 - self.buffer_len).min(rest.len());
            let (head, tail) = rest.split_at(take);
            if let Some(dst) = self.buffer.get_mut(self.buffer_len..self.buffer_len + take) {
                dst.copy_from_slice(head);
            }
            self.buffer_len = self.buffer_len.wrapping_add(take);
            rest = tail;
            if self.buffer_len == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
            }
        }
        let mut chunks = rest.chunks_exact(64);
        for chunk in &mut chunks {
            let mut block = [0_u8; 64];
            block.copy_from_slice(chunk);
            self.compress(&block);
        }
        let rem = chunks.remainder();
        if !rem.is_empty() {
            if let Some(dst) = self.buffer.get_mut(..rem.len()) {
                dst.copy_from_slice(rem);
            }
            self.buffer_len = rem.len();
        }
    }

    /// Finalizes the digest and returns the 32-byte SHA-256 hash.
    pub fn finalize(mut self) -> [u8; 32] {
        let len_bytes = self.bit_len.to_be_bytes();
        if let Some(slot) = self.buffer.get_mut(self.buffer_len) {
            *slot = 0x80;
        }
        self.buffer_len = self.buffer_len.wrapping_add(1);
        if self.buffer_len > 56 {
            if let Some(tail) = self.buffer.get_mut(self.buffer_len..) {
                tail.fill(0);
            }
            let block = self.buffer;
            self.compress(&block);
            self.buffer.fill(0);
        } else if let Some(mid) = self.buffer.get_mut(self.buffer_len..56) {
            mid.fill(0);
        }
        if let Some(tail) = self.buffer.get_mut(56..64) {
            tail.copy_from_slice(&len_bytes);
        }
        let last_block = self.buffer;
        self.compress(&last_block);

        let mut out = [0_u8; 32];
        for (slot, word) in out.chunks_exact_mut(4).zip(self.state) {
            slot.copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0_u32; 64];
        for (word, bytes) in w.iter_mut().zip(block.chunks_exact(4)) {
            if let [b0, b1, b2, b3] = *bytes {
                *word = u32::from_be_bytes([b0, b1, b2, b3]);
            }
        }
        let mut i = 16_usize;
        while i < 64 {
            let w15 = w.get(i - 15).copied().unwrap_or(0);
            let w2 = w.get(i - 2).copied().unwrap_or(0);
            let w16 = w.get(i - 16).copied().unwrap_or(0);
            let w7 = w.get(i - 7).copied().unwrap_or(0);
            let s0 = w15.rotate_right(7) ^ w15.rotate_right(18) ^ (w15 >> 3);
            let s1 = w2.rotate_right(17) ^ w2.rotate_right(19) ^ (w2 >> 10);
            let val = w16.wrapping_add(s0).wrapping_add(w7).wrapping_add(s1);
            if let Some(slot) = w.get_mut(i) {
                *slot = val;
            }
            i = i.wrapping_add(1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for (wi, ki) in w.iter().copied().zip(ROUND_CONSTANTS) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(ki)
                .wrapping_add(wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        let adds = [a, b, c, d, e, f, g, h];
        for (slot, delta) in self.state.iter_mut().zip(adds) {
            *slot = slot.wrapping_add(delta);
        }
    }
}

/// Computes the 32-byte SHA-256 digest of a byte slice.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize()
}

/// Parses a 64-character lowercase or uppercase hex SHA-256 string into bytes.
pub fn parse_sha256_hex(hex: &str) -> Option<[u8; 32]> {
    let bytes = hex.trim().as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0_u8; 32];
    for (slot, pair) in out.iter_mut().zip(bytes.chunks_exact(2)) {
        let [hi, lo] = *pair else {
            return None;
        };
        *slot = (hex_nibble(hi)? << 4) | hex_nibble(lo)?;
    }
    Some(out)
}

const fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Verifies a caller-supplied [`AtmosphericGridProvider`] by streaming its
/// canonical grid bytes through SHA-256 and comparing against
/// `expected_sha256()` (or a 64-hex `provenance().source_sha256`).
///
/// # Errors
///
/// [`PropagationError::GridIntegrityMismatch`] when no valid expected digest is
/// declared or the computed digest differs from the expected digest.
pub fn verify_provider_integrity<P: AtmosphericGridProvider + ?Sized>(
    provider: &P,
) -> Result<[u8; 32], PropagationError> {
    let expected = provider
        .expected_sha256()
        .or_else(|| {
            provider
                .provenance()
                .and_then(|p| parse_sha256_hex(p.source_sha256))
        })
        .ok_or(PropagationError::GridIntegrityMismatch)?;
    let mut hasher = Sha256::new();
    provider.visit_grid_bytes(&mut |chunk| hasher.update(chunk));
    let actual = hasher.finalize();
    if actual == expected {
        Ok(actual)
    } else {
        Err(PropagationError::GridIntegrityMismatch)
    }
}

/// Borrowed wrapper guaranteeing an [`AtmosphericGridProvider`] passed
/// SHA-256 verification on construction.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedGridProvider<'a, P: ?Sized> {
    provider: &'a P,
    digest: [u8; 32],
}

impl<'a, P: AtmosphericGridProvider + ?Sized> VerifiedGridProvider<'a, P> {
    /// Verifies `provider`'s SHA-256 integrity and returns a verified handle.
    ///
    /// # Errors
    ///
    /// [`PropagationError::GridIntegrityMismatch`] if integrity verification fails.
    pub fn new(provider: &'a P) -> Result<Self, PropagationError> {
        let digest = verify_provider_integrity(provider)?;
        Ok(Self { provider, digest })
    }

    /// Underlying atmospheric grid provider.
    pub const fn provider(&self) -> &'a P {
        self.provider
    }

    /// Verified SHA-256 digest of the provider's grid bytes.
    pub const fn verified_sha256(&self) -> [u8; 32] {
        self.digest
    }
}
