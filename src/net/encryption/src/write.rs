use aes::cipher::consts::U1;
use aes::cipher::{Array, BlockModeEncrypt, KeyIvInit};
use aes::Aes128;
use cfb8::Encryptor;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

/// A wrapper around a writer that encrypts outgoing bytes using AES/CFB8, if configured.
pub struct EncryptedWriter<Writer> {
    writer: Writer,
    cipher: Option<Encryptor<Aes128>>,
}

impl<Writer> EncryptedWriter<Writer> {
    /// Sets the internal AES cipher to use the specified key.
    ///
    /// # Parameters
    /// - `key`: The key to use for the AES cipher.
    pub fn update_cipher(&mut self, key: &[u8]) {
        self.cipher = Some(Encryptor::new_from_slices(key, key).unwrap());
    }

    pub fn encrypt_buf(&mut self, buf: &mut [u8]) {
        if let Some(enc) = self.cipher.as_mut() {
            for b in buf.chunks_mut(1) {
                let block: &mut Array<u8, U1> = b.try_into().unwrap();
                enc.encrypt_block(block);
            }
        }
    }
}

impl<Writer> From<Writer> for EncryptedWriter<Writer> {
    fn from(writer: Writer) -> Self {
        Self {
            writer,
            cipher: None,
        }
    }
}

impl<Writer: AsyncWrite + Unpin> AsyncWrite for EncryptedWriter<Writer> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut self.writer).poll_write(cx, buf)
    }

    // Wrap the internal writer's poll_flush and poll_shutdown methods
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.writer).poll_shutdown(cx)
    }
}
