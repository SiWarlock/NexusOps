//! Length-prefixed frame codec (§6.4 [LOCKED — ADR-004]).
//!
//! Every wire frame is a **4-byte big-endian length prefix + JSON body** (newline-framing was
//! dropped per ADR-004). The declared length is validated against [`MAX_FRAME_SIZE`] from the
//! prefix **before** the body buffer is allocated, so an oversized declared length can never
//! drive an oversized allocation (the anti-DoS pin).

use super::IpcError;

/// Fixed anti-DoS cap on a single frame's JSON body (§6.4). 8 MiB: the ui's outbound frames
/// are small and large responses are paginated, so this bounds a hostile/buggy peer without
/// constraining real traffic. A frame whose 4-byte prefix declares more is rejected by
/// [`decode_len`] before any read buffer is sized.
pub const MAX_FRAME_SIZE: usize = 8 * 1024 * 1024;

/// Encode a JSON `body` as a wire frame: a 4-byte big-endian length prefix + the body. Refuses
/// a body larger than [`MAX_FRAME_SIZE`] (symmetry with the read-side bound).
pub fn encode_frame(body: &[u8]) -> Result<Vec<u8>, IpcError> {
    if body.len() > MAX_FRAME_SIZE {
        return Err(IpcError::FrameTooLarge {
            declared: body.len(),
            max: MAX_FRAME_SIZE,
        });
    }
    let mut out = Vec::with_capacity(4 + body.len());
    // body.len() <= MAX_FRAME_SIZE (8 MiB) << u32::MAX, so the cast cannot truncate.
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
    Ok(out)
}

/// Decode the declared body length from a frame's 4-byte big-endian prefix, rejecting a length
/// over [`MAX_FRAME_SIZE`] (§6.4). Called by the reader **before** allocating the body buffer,
/// so an oversized declared length is rejected without ever sizing an oversized allocation.
pub fn decode_len(prefix: &[u8; 4]) -> Result<usize, IpcError> {
    let len = u32::from_be_bytes(*prefix) as usize;
    // a 0-length body is a structurally valid frame at the CODEC layer; semantic rejection of
    // an empty / non-JSON body is the L2 handshake+parse layer's job, not the framing layer's.
    if len > MAX_FRAME_SIZE {
        return Err(IpcError::FrameTooLarge {
            declared: len,
            max: MAX_FRAME_SIZE,
        });
    }
    Ok(len)
}
