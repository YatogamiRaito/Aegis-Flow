use bytes::{Buf, Bytes, BytesMut};
use h3::quic::{
    BidiStream, Connection, ConnectionErrorIncoming, OpenStreams, RecvStream, SendStream,
    StreamErrorIncoming, StreamId, WriteBuf,
};
use s2n_quic::stream::{BidirectionalStream, ReceiveStream, SendStream as S2nSendStreamStream};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub struct S2nConnection(pub s2n_quic::Connection);

pub struct S2nBidiStream {
    stream: BidirectionalStream,
    buf: BytesMut,
}

impl S2nBidiStream {
    pub fn new(stream: BidirectionalStream) -> Self {
        Self {
            stream,
            buf: BytesMut::new(),
        }
    }
}

pub struct S2nSendStream {
    stream: S2nSendStreamStream,
    buf: BytesMut,
}

pub struct S2nRecvStream(pub ReceiveStream);

pub struct S2nOpenStreams;

impl<B: Buf> BidiStream<B> for S2nBidiStream {
    type SendStream = S2nSendStream;
    type RecvStream = S2nRecvStream;

    fn split(self) -> (Self::SendStream, Self::RecvStream) {
        let (rx, tx) = self.stream.split();
        (
            S2nSendStream {
                stream: tx,
                buf: self.buf,
            },
            S2nRecvStream(rx),
        )
    }
}

impl RecvStream for S2nBidiStream {
    type Buf = Bytes;

    fn poll_data(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<Self::Buf>, StreamErrorIncoming>> {
        let mut buf = [0u8; 8192];
        let mut read_buf = ReadBuf::new(&mut buf);
        match Pin::new(&mut self.stream).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                if read_buf.filled().is_empty() {
                    Poll::Ready(Ok(None))
                } else {
                    Poll::Ready(Ok(Some(Bytes::copy_from_slice(read_buf.filled()))))
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(StreamErrorIncoming::Unknown(Box::new(e)))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn stop_sending(&mut self, _error_code: u64) {}

    fn recv_id(&self) -> StreamId {
        unsafe { std::mem::transmute(0u64) }
    }
}

impl<B: Buf> SendStream<B> for S2nBidiStream {
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), StreamErrorIncoming>> {
        if !self.buf.is_empty() {
            match Pin::new(&mut self.stream).poll_write(cx, &self.buf) {
                Poll::Ready(Ok(n)) => {
                    self.buf.advance(n);
                    if self.buf.is_empty() {
                        Poll::Ready(Ok(()))
                    } else {
                        Poll::Pending
                    }
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(StreamErrorIncoming::Unknown(Box::new(e)))),
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_finish(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), StreamErrorIncoming>> {
        match Pin::new(&mut self.stream).poll_flush(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(StreamErrorIncoming::Unknown(Box::new(e)))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn send_data<T: Into<WriteBuf<B>>>(&mut self, data: T) -> Result<(), StreamErrorIncoming> {
        let mut wb = data.into();
        while wb.has_remaining() {
            let chunk = wb.chunk();
            self.buf.extend_from_slice(chunk);
            wb.advance(chunk.len());
        }
        Ok(())
    }

    fn reset(&mut self, _reset_code: u64) {}

    fn send_id(&self) -> StreamId {
        unsafe { std::mem::transmute(0u64) }
    }
}

impl RecvStream for S2nRecvStream {
    type Buf = Bytes;

    fn poll_data(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<Self::Buf>, StreamErrorIncoming>> {
        let mut buf = [0u8; 8192];
        let mut read_buf = ReadBuf::new(&mut buf);
        match Pin::new(&mut self.0).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                if read_buf.filled().is_empty() {
                    Poll::Ready(Ok(None))
                } else {
                    Poll::Ready(Ok(Some(Bytes::copy_from_slice(read_buf.filled()))))
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(StreamErrorIncoming::Unknown(Box::new(e)))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn stop_sending(&mut self, _error_code: u64) {}

    fn recv_id(&self) -> StreamId {
        unsafe { std::mem::transmute(0u64) }
    }
}

impl<B: Buf> SendStream<B> for S2nSendStream {
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), StreamErrorIncoming>> {
        if !self.buf.is_empty() {
            match Pin::new(&mut self.stream).poll_write(cx, &self.buf) {
                Poll::Ready(Ok(n)) => {
                    self.buf.advance(n);
                    if self.buf.is_empty() {
                        Poll::Ready(Ok(()))
                    } else {
                        Poll::Pending
                    }
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(StreamErrorIncoming::Unknown(Box::new(e)))),
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_finish(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), StreamErrorIncoming>> {
        match Pin::new(&mut self.stream).poll_flush(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(StreamErrorIncoming::Unknown(Box::new(e)))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn send_data<T: Into<WriteBuf<B>>>(&mut self, data: T) -> Result<(), StreamErrorIncoming> {
        let mut wb = data.into();
        while wb.has_remaining() {
            let chunk = wb.chunk();
            self.buf.extend_from_slice(chunk);
            wb.advance(chunk.len());
        }
        Ok(())
    }

    fn reset(&mut self, _reset_code: u64) {}

    fn send_id(&self) -> StreamId {
        unsafe { std::mem::transmute(0u64) }
    }
}

impl<B: Buf> OpenStreams<B> for S2nConnection {
    type BidiStream = S2nBidiStream;
    type SendStream = S2nSendStream;

    fn poll_open_bidi(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<Self::BidiStream, StreamErrorIncoming>> {
        Poll::Ready(Err(StreamErrorIncoming::Unknown(Box::new(
            std::io::Error::new(std::io::ErrorKind::Other, "Not implemented natively via poll by s2n, would need background task wrapper"),
        ))))
    }

    fn poll_open_send(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<Self::SendStream, StreamErrorIncoming>> {
        Poll::Ready(Err(StreamErrorIncoming::Unknown(Box::new(
            std::io::Error::new(std::io::ErrorKind::Other, "Not implemented natively via poll by s2n"),
        ))))
    }

    fn close(&mut self, _code: h3::error::Code, _reason: &[u8]) {}
}

impl<B: Buf> Connection<B> for S2nConnection {
    type RecvStream = S2nRecvStream;
    type OpenStreams = S2nOpenStreams;

    fn poll_accept_bidi(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<Self::BidiStream, ConnectionErrorIncoming>> {
        Poll::Ready(Err(ConnectionErrorIncoming::InternalError(
            "Manual accept expected in proxy server".to_string(),
        )))
    }

    fn poll_accept_recv(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<Self::RecvStream, ConnectionErrorIncoming>> {
        Poll::Ready(Err(ConnectionErrorIncoming::InternalError(
            "Manual accept expected in proxy server".to_string(),
        )))
    }

    fn opener(&self) -> Self::OpenStreams {
        S2nOpenStreams
    }
}

impl<B: Buf> OpenStreams<B> for S2nOpenStreams {
    type BidiStream = S2nBidiStream;
    type SendStream = S2nSendStream;

    fn poll_open_bidi(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<Self::BidiStream, StreamErrorIncoming>> {
        Poll::Ready(Err(StreamErrorIncoming::Unknown(Box::new(
            std::io::Error::new(std::io::ErrorKind::Other, "Not implemented"),
        ))))
    }

    fn poll_open_send(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<Self::SendStream, StreamErrorIncoming>> {
        Poll::Ready(Err(StreamErrorIncoming::Unknown(Box::new(
            std::io::Error::new(std::io::ErrorKind::Other, "Not implemented"),
        ))))
    }

    fn close(&mut self, _code: h3::error::Code, _reason: &[u8]) {}
}
