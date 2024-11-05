pub mod codec;
pub mod endpoint;
pub mod event_mgr;
pub mod master_port;
pub mod media_types;
pub mod port;
pub mod rtcp;
pub mod rtp;
pub mod sdp;
pub mod signatures;
pub mod stream;
pub mod transport;
pub mod wav_streamer;
pub mod wave;

pub use codec::*;
pub use endpoint::*;
pub use event_mgr::*;
pub use master_port::*;
pub use media_types::*;
pub use port::*;
pub use rtcp::*;
pub use rtp::*;
pub use sdp::*;
pub use signatures::*;
pub use stream::*;
pub use transport::*;
pub use wav_streamer::*;
pub use wave::*;

pub type Fourcc = &'static [u8; 4];

pub fn fourcc_to_int(sig: Fourcc) -> u32 {
    let mut s = 0u32;
    sig.iter()
        .enumerate()
        .for_each(|(i, c)| s |= (*c as u32) << (i * 8));

    s
}
