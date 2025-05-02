#![cfg(feature = "l2cap")]

use std::{
    any::Any,
    ffi::c_void,
    fmt::Debug,
    mem::ManuallyDrop,
    ops::Deref,
    ptr::null,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll, Waker},
};

use core_foundation_sys::{
    runloop::{kCFRunLoopCommonModes, CFRunLoopGetCurrent, CFRunLoopRun, CFRunLoopStop},
    stream::{
        kCFStreamEventCanAcceptBytes, kCFStreamEventEndEncountered, kCFStreamEventErrorOccurred,
        kCFStreamEventHasBytesAvailable, kCFStreamEventOpenCompleted, CFReadStreamRef, CFStreamClientContext,
        CFStreamEventType, CFWriteStreamRef,
    },
    string::CFStringRef,
};
use futures_io::{AsyncRead, AsyncWrite};
use objc_id::ShareId;
use tracing::{debug, trace};

use crate::corebluetooth::types::{NSInputStream, NSOutputStream};

use cf_stream::{CFReadStream, CFWriteStream};
mod cf_stream;

#[derive(Debug)]
pub struct FuturesCFStream {
    context: Arc<CFAsyncStreamContext>,
    input: ShareId<NSInputStream>,
    output: ShareId<NSOutputStream>,
}

#[derive(Debug)]
struct CFAsyncStreamContext {
    event: AtomicUsize,
    waker: Mutex<Option<Waker>>,
    _stream: Box<dyn Any + Send + Sync + 'static>,
}

impl FuturesCFStream {
    pub fn cf_stream(
        stream: impl Debug + Any + Send + Sync + 'static,
        input: ShareId<NSInputStream>,
        output: ShareId<NSOutputStream>,
    ) -> Self {
        let context = Arc::new(CFAsyncStreamContext {
            event: AtomicUsize::default(),
            waker: Default::default(),
            _stream: Box::new(stream),
        });

        {
            let input = input.clone();
            let output = output.clone();
            let context = context.clone();
            #[cfg(not(feature = "tokio"))]
            std::thread::spawn(move || {
                in_runloop(context, input, output);
            });
            #[cfg(feature = "tokio")]
            tokio::task::spawn_blocking(move || {
                in_runloop(context, input, output);
            });
        }

        Self { context, input, output }
    }

    fn set_waker(&self, cx: &mut Context<'_>) {
        let mut waker = self.context.waker.lock().unwrap();
        assert!(waker.is_none());
        *waker = Some(cx.waker().clone());
    }
}

fn in_runloop(context: Arc<CFAsyncStreamContext>, input: ShareId<NSInputStream>, output: ShareId<NSOutputStream>) {
    let current_loop = unsafe { CFRunLoopGetCurrent() };

    let mut cf_context = Box::new(CFStreamClientContext {
        version: 0,
        info: Arc::into_raw(context.clone()).cast_mut().cast(),
        retain: cfstream_context_clone,
        release: cfstream_context_drop,
        copyDescription: cfstream_context_description,
    });

    const REGISTERED_EVENTS: CFStreamEventType = kCFStreamEventOpenCompleted
        | kCFStreamEventHasBytesAvailable
        | kCFStreamEventCanAcceptBytes
        | kCFStreamEventEndEncountered
        | kCFStreamEventErrorOccurred;

    debug!("Registering for callbacks on {:?}", context._stream);
    let mut read_stream = CFReadStream::new((input.deref() as *const NSInputStream).cast_mut().cast());
    let opened = unsafe {
        assert_eq!(
            read_stream.set_client(REGISTERED_EVENTS, cfreadstream_callback, &mut *cf_context),
            1,
            "Read Stream set client not supported"
        );

        read_stream.schedule(current_loop, kCFRunLoopCommonModes);
        read_stream.open()
    };
    if !opened {
        tracing::error!("Failed to open read stream.");
        return;
    }

    let mut write_stream: CFWriteStream =
        CFWriteStream::new((output.deref() as *const NSOutputStream).cast_mut().cast());
    let opened = unsafe {
        assert_eq!(
            write_stream.set_client(REGISTERED_EVENTS, cfwritestream_callback, &mut *cf_context),
            1,
            "Write Stream set client not supported"
        );
        write_stream.schedule(current_loop, kCFRunLoopCommonModes);
        write_stream.open()
    };
    if !opened {
        tracing::error!("Failed to open write stream.");
        return;
    }
    tracing::trace!("Running run loop");

    // This blocks until CFRunLoopStop is called (usually from the callback)
    unsafe { CFRunLoopRun() };
}

impl AsyncRead for FuturesCFStream {
    fn poll_read(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        tracing::trace!("Read Polling L2cap {}", self.input.has_bytes_available());
        if self.input.has_bytes_available() {
            match self.input.read(buf) {
                -1 => todo!(),
                i if i < 0 => unreachable!(),
                bytes_read => Poll::Ready(Ok(bytes_read.try_into().unwrap())),
            }
        } else {
            self.set_waker(cx);
            Poll::Pending
        }
    }
}

impl AsyncWrite for FuturesCFStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.output.has_space_available() {
            match self.output.write(buf) {
                -1 => todo!(),
                i if i < 0 => unreachable!(),
                bytes_written => Poll::Ready(Ok(bytes_written.try_into().unwrap())),
            }
        } else {
            self.set_waker(cx);
            Poll::Pending
        }
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

extern "C" fn cfwritestream_callback(
    _stream: CFWriteStreamRef,
    event_type: CFStreamEventType,
    client_call_back_info: *mut c_void,
) {
    trace!("Write Stream Callback");
    cfstream_callback(event_type, client_call_back_info)
}

extern "C" fn cfreadstream_callback(
    _stream: CFReadStreamRef,
    event_type: CFStreamEventType,
    client_call_back_info: *mut c_void,
) {
    trace!("Read Stream Callback");
    cfstream_callback(event_type, client_call_back_info)
}

fn cfstream_callback(event_type: CFStreamEventType, client_call_back_info: *mut c_void) {
    trace!("Stream Callback");
    if client_call_back_info.is_null() {
        // This should never happen.
        panic!("Callback client info is null")
    }

    let context: ManuallyDrop<Arc<CFAsyncStreamContext>> =
        ManuallyDrop::new(unsafe { Arc::from_raw(client_call_back_info.cast()) });

    context.event.fetch_or(event_type, Ordering::Relaxed);

    let waker = context.waker.lock().unwrap().take();
    if let Some(waker) = waker {
        waker.wake();
    }

    if event_type & kCFStreamEventEndEncountered != 0 {
        unsafe { CFRunLoopStop(CFRunLoopGetCurrent()) }
    }

    if event_type & kCFStreamEventErrorOccurred != 0 {
        unsafe { CFRunLoopStop(CFRunLoopGetCurrent()) }
    }
}

extern "C" fn cfstream_context_clone(context: *const c_void) -> *const c_void {
    trace!("Cloning CFStream Context",);
    unsafe { Arc::increment_strong_count(context) };
    context
}

extern "C" fn cfstream_context_drop(context: *const c_void) {
    trace!("Dropping CFStream Context");
    unsafe { Arc::decrement_strong_count(context) }
}

extern "C" fn cfstream_context_description(_context: *const c_void) -> CFStringRef {
    trace!("context_description");
    null()
}
