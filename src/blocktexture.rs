pub trait Block: Sized {
    type Bytes: AsRef<[u8]>;
    // = [u8; 8], etc. Many thanks to @kornel@mastodon.social
    const SIZE: usize;
    const WIDTH: usize = 4;
    const HEIGHT: usize = 4;

    fn to_bytes(&self) -> Self::Bytes;
    fn from_bytes(bytes: &Self::Bytes) -> Self;
}

struct BlockTexture<B>
where
    B: Block,
{
    width: usize,
    height: usize,
    blocks: Vec<B>,
}
