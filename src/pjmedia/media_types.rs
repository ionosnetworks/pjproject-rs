use pjproject_sys as pj;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum PjMediaDir {
    /** None */
    None = 0,
    /** Encoding (outgoing to network) stream, also known as capture */
    Encoding = 1,
    /** Decoding (incoming from network) stream, also known as playback. */
    Decoding = 2,
    /** Incoming and outgoing stream, same as PJMEDIA_DIR_CAPTURE_PLAYBACK */
    EncodingDecoding = 3,
}

impl PjMediaDir {
    /** None */
    pub const NONE: PjMediaDir = PjMediaDir::None;
    /** Encoding (outgoing to network) stream, also known as capture */
    pub const ENCODING: PjMediaDir = PjMediaDir::Encoding;
    /** Same as encoding direction. */
    pub const CAPTURE: PjMediaDir = PjMediaDir::Encoding;
    /** Decoding (incoming from network) stream, also known as playback. */
    pub const DECODING: PjMediaDir = PjMediaDir::Decoding;
    /** Same as decoding. */
    pub const PLAYBACK: PjMediaDir = PjMediaDir::Decoding;
    /** Same as decoding. */
    pub const RENDER: PjMediaDir = PjMediaDir::Decoding;
    /** Incoming and outgoing stream, same as PJMEDIA_DIR_CAPTURE_PLAYBACK */
    pub const ENCODING_DECODING: PjMediaDir = PjMediaDir::EncodingDecoding;
    /** Same as ENCODING_DECODING */
    pub const CAPTURE_PLAYBACK: PjMediaDir = PjMediaDir::EncodingDecoding;
    /** Same as ENCODING_DECODING */
    pub const CAPTURE_RENDER: PjMediaDir = PjMediaDir::EncodingDecoding;
}

impl From<PjMediaDir> for pj::pjmedia_dir {
    fn from(value: PjMediaDir) -> Self {
        value as _
    }
}
