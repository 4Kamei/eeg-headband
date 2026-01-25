#![no_std]

pub const EEG_DATA_SERVICE_UUID: [u8; 16] = [
    255, 77, 189, 23, 34, 96, 77, 13, 167, 102, 45, 228, 119, 88, 43, 141,
];

#[allow(dead_code)]
#[unsafe(link_section = ".shared_ram.ble_queue")]
pub static BLE_QUEUE: crate::ring_buffer::UninitRingBuffer<u64, 1024> =
    crate::ring_buffer::UninitRingBuffer::new();

pub mod ring_buffer {

    use core::{cell::UnsafeCell, sync::atomic::AtomicBool};
    use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch};
    use heapless::spsc;

    unsafe impl<T: Sync + Copy, const N: usize> Sync for UninitRingBuffer<T, N> {}

    pub struct RingBufferProducer<'a, T, const M: usize> {
        sender: spsc::Producer<'a, T>,
        signal: watch::Sender<'a, CriticalSectionRawMutex, (), M>,
    }

    impl<'a, T, const M: usize> RingBufferProducer<'a, T, M> {
        pub fn send(&mut self, value: T) -> Result<(), T> {
            let res = self.sender.enqueue(value);
            if res.is_ok() {
                self.signal.send(())
            }
            res
        }
    }

    pub struct RingBufferConsumer<'a, T, const M: usize> {
        receiver: spsc::Consumer<'a, T>,
        signal: watch::Receiver<'a, CriticalSectionRawMutex, (), M>,
    }

    impl<'a, T, const M: usize> RingBufferConsumer<'a, T, M> {
        pub async fn recv(&mut self) -> T {
            loop {
                self.signal.changed().await;
                if self.receiver.ready() {
                    return unsafe { self.receiver.dequeue_unchecked() };
                };
            }
        }
    }
    /// Ring buffer that has not yet been initialized
    pub struct UninitRingBuffer<T: Copy, const N: usize> {
        ring_buffer: UnsafeCell<spsc::Queue<T, N>>,
    }

    impl<T: Copy, const N: usize> UninitRingBuffer<T, N> {
        pub const fn new() -> Self {
            Self {
                ring_buffer: UnsafeCell::new(spsc::Queue::new()),
            }
        }

        /// Gets the sender part of this channel. Safety: This should only be called once
        pub unsafe fn get_sender(&self) -> spsc::Producer<'_, T> {
            unsafe { &mut *(self.ring_buffer.get() as *mut spsc::Queue<T, N>) }
                .split()
                .0
        }

        /// Gets the sender part of this channel. Safety: This should only be called once
        pub unsafe fn get_receiver(&self) -> spsc::Consumer<'_, T> {
            unsafe { &mut *(self.ring_buffer.get() as *mut spsc::Queue<T, N>) }
                .split()
                .1
        }

        pub unsafe fn get_receiver_with_signal<'a, const M: usize>(
            &'a self,
            signal: watch::Receiver<'a, CriticalSectionRawMutex, (), M>,
        ) -> RingBufferConsumer<'a, T, M> {
            RingBufferConsumer {
                receiver: unsafe { self.get_receiver() },
                signal,
            }
        }

        pub unsafe fn get_sender_with_signal<'a, const M: usize>(
            &'a self,
            signal: watch::Sender<'a, CriticalSectionRawMutex, (), M>,
        ) -> RingBufferProducer<'a, T, M> {
            RingBufferProducer {
                sender: unsafe { self.get_sender() },
                signal,
            }
        }
    }
}
