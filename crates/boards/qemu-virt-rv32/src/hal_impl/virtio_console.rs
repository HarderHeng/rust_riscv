//! Minimal modern virtio-console driver for QEMU virtio-mmio.
//!
//! The driver intentionally uses polling for queue completion. The kernel's
//! PLIC path still enables the virtio IRQ so `wfi` wakes for receive traffic;
//! the used ring is then consumed by `try_getc`.

use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{fence, Ordering};

use hal::SerialPort;
use spin::Mutex;

const MAGIC: u32 = 0x7472_6976;
const DEVICE_CONSOLE: u32 = 3;
const VERSION_MODERN: u32 = 2;
const FEATURE_VERSION_1: u64 = 1 << 32;
const QUEUE_RX: u32 = 0;
const QUEUE_TX: u32 = 1;
const QUEUE_CTRL_RX: u32 = 2;
const QUEUE_CTRL_TX: u32 = 3;

const CTRL_DEVICE_READY: u16 = 0;
const CTRL_PORT_ADD: u16 = 1;
const CTRL_PORT_READY: u16 = 3;
const CTRL_PORT_OPEN: u16 = 6;

const STATUS_ACKNOWLEDGE: u32 = 1;
const STATUS_DRIVER: u32 = 2;
const STATUS_DRIVER_OK: u32 = 4;
const STATUS_FEATURES_OK: u32 = 8;
const STATUS_FAILED: u32 = 128;

const DESC_F_WRITE: u16 = 2;
const QUEUE_SIZE: usize = 8;
const BUFFER_SIZE: usize = 256;

const REG_MAGIC: usize = 0x000;
const REG_VERSION: usize = 0x004;
const REG_DEVICE_ID: usize = 0x008;
const REG_DEVICE_FEATURES: usize = 0x010;
const REG_DEVICE_FEATURES_SEL: usize = 0x014;
const REG_DRIVER_FEATURES: usize = 0x020;
const REG_DRIVER_FEATURES_SEL: usize = 0x024;
const REG_QUEUE_SEL: usize = 0x030;
const REG_QUEUE_NUM_MAX: usize = 0x034;
const REG_QUEUE_NUM: usize = 0x038;
const REG_QUEUE_READY: usize = 0x044;
const REG_QUEUE_NOTIFY: usize = 0x050;
const REG_INTERRUPT_STATUS: usize = 0x060;
const REG_INTERRUPT_ACK: usize = 0x064;
const REG_STATUS: usize = 0x070;
const REG_QUEUE_DESC_LOW: usize = 0x080;
const REG_QUEUE_DESC_HIGH: usize = 0x084;
const REG_QUEUE_DRIVER_LOW: usize = 0x090;
const REG_QUEUE_DRIVER_HIGH: usize = 0x094;
const REG_QUEUE_DEVICE_LOW: usize = 0x0A0;
const REG_QUEUE_DEVICE_HIGH: usize = 0x0A4;

#[repr(C)]
#[derive(Copy, Clone)]
struct Descriptor {
    address: u64,
    length: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
struct Available {
    flags: u16,
    index: u16,
    ring: [u16; QUEUE_SIZE],
    used_event: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct UsedElement {
    id: u32,
    length: u32,
}

#[repr(C)]
struct Used {
    flags: u16,
    index: u16,
    ring: [UsedElement; QUEUE_SIZE],
    available_event: u16,
}

#[repr(C, align(4096))]
struct QueueMemory {
    descriptors: [Descriptor; QUEUE_SIZE],
    available: Available,
    // Keep each used ring on its own page for simple device inspection and
    // future legacy transport support.
    legacy_padding: [u8; 4096 - (QUEUE_SIZE * 16 + 22)],
    used: Used,
    buffer: [u8; BUFFER_SIZE],
}

impl QueueMemory {
    const ZERO_DESCRIPTOR: Descriptor = Descriptor {
        address: 0,
        length: 0,
        flags: 0,
        next: 0,
    };

    const ZERO_USED: UsedElement = UsedElement { id: 0, length: 0 };

    const fn new() -> Self {
        Self {
            descriptors: [Self::ZERO_DESCRIPTOR; QUEUE_SIZE],
            available: Available {
                flags: 0,
                index: 0,
                ring: [0; QUEUE_SIZE],
                used_event: 0,
            },
            legacy_padding: [0; 4096 - (QUEUE_SIZE * 16 + 22)],
            used: Used {
                flags: 0,
                index: 0,
                ring: [Self::ZERO_USED; QUEUE_SIZE],
                available_event: 0,
            },
            buffer: [0; BUFFER_SIZE],
        }
    }
}

struct State {
    tx: QueueMemory,
    rx: QueueMemory,
    ctrl_tx: QueueMemory,
    ctrl_rx: QueueMemory,
    tx_size: u16,
    rx_size: u16,
    ctrl_tx_size: u16,
    ctrl_rx_size: u16,
    tx_last_used: u16,
    rx_last_used: u16,
    ctrl_tx_last_used: u16,
    ctrl_rx_last_used: u16,
    rx_length: usize,
    rx_offset: usize,
    port_ready: bool,
    ready: bool,
}

impl State {
    const fn new() -> Self {
        Self {
            tx: QueueMemory::new(),
            rx: QueueMemory::new(),
            ctrl_tx: QueueMemory::new(),
            ctrl_rx: QueueMemory::new(),
            tx_size: 0,
            rx_size: 0,
            ctrl_tx_size: 0,
            ctrl_rx_size: 0,
            tx_last_used: 0,
            rx_last_used: 0,
            ctrl_tx_last_used: 0,
            ctrl_rx_last_used: 0,
            rx_length: 0,
            rx_offset: 0,
            port_ready: false,
            ready: false,
        }
    }
}

/// A virtio-console device on a virtio-mmio slot.
pub struct VirtioConsole {
    base: Mutex<usize>,
    state: Mutex<State>,
}

impl VirtioConsole {
    /// Creates a virtio-console handle at `base`.
    pub const fn new(base: usize) -> Self {
        Self {
            base: Mutex::new(base),
            state: Mutex::new(State::new()),
        }
    }

    #[inline]
    fn read_at(base: usize, offset: usize) -> u32 {
        unsafe { read_volatile((base + offset) as *const u32) }
    }

    #[inline]
    fn write_at(base: usize, offset: usize, value: u32) {
        unsafe { write_volatile((base + offset) as *mut u32, value) }
    }

    #[inline]
    fn read(&self, offset: usize) -> u32 {
        Self::read_at(*self.base.lock(), offset)
    }

    #[inline]
    fn write(&self, offset: usize, value: u32) {
        Self::write_at(*self.base.lock(), offset, value)
    }

    fn set_status(&self, status: u32) {
        self.write(REG_STATUS, status);
    }

    fn fail(&self) {
        self.set_status(self.read(REG_STATUS) | STATUS_FAILED);
    }

    fn ack_interrupt(&self) {
        let status = self.read(REG_INTERRUPT_STATUS);
        if status != 0 {
            self.write(REG_INTERRUPT_ACK, status);
        }
    }

    /// Public IRQ acknowledge for the board console IRQ handler.
    pub fn ack_pending_interrupt(&self) {
        self.ack_interrupt();
    }

    fn configure_queue(&self, index: u32, memory: &mut QueueMemory) -> Option<u16> {
        self.write(REG_QUEUE_SEL, index);
        let max = self.read(REG_QUEUE_NUM_MAX) as usize;
        if max == 0 {
            return None;
        }
        let size = max.min(QUEUE_SIZE) as u16;
        self.write(REG_QUEUE_NUM, size as u32);

        let descriptor = memory.descriptors.as_ptr() as usize as u64;
        let available = &memory.available as *const Available as usize as u64;
        let used = &memory.used as *const Used as usize as u64;

        self.write(REG_QUEUE_DESC_LOW, descriptor as u32);
        self.write(REG_QUEUE_DESC_HIGH, (descriptor >> 32) as u32);
        self.write(REG_QUEUE_DRIVER_LOW, available as u32);
        self.write(REG_QUEUE_DRIVER_HIGH, (available >> 32) as u32);
        self.write(REG_QUEUE_DEVICE_LOW, used as u32);
        self.write(REG_QUEUE_DEVICE_HIGH, (used >> 32) as u32);
        self.write(REG_QUEUE_READY, 1);
        Some(size)
    }

    fn post_receive_buffer(&self, state: &mut State) {
        let descriptor = &mut state.rx.descriptors[0];
        descriptor.address = state.rx.buffer.as_ptr() as usize as u64;
        descriptor.length = BUFFER_SIZE as u32;
        descriptor.flags = DESC_F_WRITE;
        descriptor.next = 0;

        let slot = (state.rx.available.index as usize) % state.rx_size as usize;
        unsafe {
            write_volatile(&mut state.rx.available.ring[slot], 0);
            fence(Ordering::SeqCst);
            write_volatile(
                &mut state.rx.available.index,
                state.rx.available.index.wrapping_add(1),
            );
        }
        self.write(REG_QUEUE_NOTIFY, QUEUE_RX);
    }

    fn post_control_receive_buffer(&self, state: &mut State) {
        let buffer_address = state.ctrl_rx.buffer.as_ptr() as usize as u64;
        let descriptor = &mut state.ctrl_rx.descriptors[0];
        descriptor.address = buffer_address;
        descriptor.length = 8;
        descriptor.flags = DESC_F_WRITE;
        descriptor.next = 0;

        let slot = (state.ctrl_rx.available.index as usize) % state.ctrl_rx_size as usize;
        unsafe {
            write_volatile(&mut state.ctrl_rx.available.ring[slot], 0);
            fence(Ordering::SeqCst);
            write_volatile(
                &mut state.ctrl_rx.available.index,
                state.ctrl_rx.available.index.wrapping_add(1),
            );
        }
        self.write(REG_QUEUE_NOTIFY, QUEUE_CTRL_RX);
    }

    fn send_control(&self, state: &mut State, id: u32, event: u16, value: u16) {
        state.ctrl_tx.buffer[0..4].copy_from_slice(&id.to_le_bytes());
        state.ctrl_tx.buffer[4..6].copy_from_slice(&event.to_le_bytes());
        state.ctrl_tx.buffer[6..8].copy_from_slice(&value.to_le_bytes());

        let buffer_address = state.ctrl_tx.buffer.as_ptr() as usize as u64;
        let descriptor = &mut state.ctrl_tx.descriptors[0];
        descriptor.address = buffer_address;
        descriptor.length = 8;
        descriptor.flags = 0;
        descriptor.next = 0;

        let slot = (state.ctrl_tx.available.index as usize) % state.ctrl_tx_size as usize;
        unsafe {
            write_volatile(&mut state.ctrl_tx.available.ring[slot], 0);
            fence(Ordering::SeqCst);
            write_volatile(
                &mut state.ctrl_tx.available.index,
                state.ctrl_tx.available.index.wrapping_add(1),
            );
        }
        self.write(REG_QUEUE_NOTIFY, QUEUE_CTRL_TX);

        while unsafe { read_volatile(&state.ctrl_tx.used.index) } == state.ctrl_tx_last_used {
            core::hint::spin_loop();
        }
        state.ctrl_tx_last_used = unsafe { read_volatile(&state.ctrl_tx.used.index) };
        self.ack_interrupt();
    }

    fn poll_control(&self, state: &mut State) {
        let used_index = unsafe { read_volatile(&state.ctrl_rx.used.index) };
        if used_index == state.ctrl_rx_last_used {
            return;
        }

        let slot = (state.ctrl_rx_last_used as usize) % state.ctrl_rx_size as usize;
        let id = u32::from_le_bytes([
            state.ctrl_rx.buffer[0],
            state.ctrl_rx.buffer[1],
            state.ctrl_rx.buffer[2],
            state.ctrl_rx.buffer[3],
        ]);
        let event = u16::from_le_bytes([state.ctrl_rx.buffer[4], state.ctrl_rx.buffer[5]]);
        let value = u16::from_le_bytes([state.ctrl_rx.buffer[6], state.ctrl_rx.buffer[7]]);
        let _length = unsafe { read_volatile(&state.ctrl_rx.used.ring[slot].length) };
        state.ctrl_rx_last_used = state.ctrl_rx_last_used.wrapping_add(1);
        self.post_control_receive_buffer(state);

        if event == CTRL_PORT_ADD {
            self.send_control(state, id, CTRL_PORT_READY, 1);
            self.send_control(state, id, CTRL_PORT_OPEN, 1);
            state.port_ready = true;
        } else if event == CTRL_DEVICE_READY {
            state.port_ready = value != 0;
        }
        self.ack_interrupt();
    }

    fn ensure_port_ready(&self, state: &mut State) -> bool {
        if state.port_ready {
            return true;
        }

        for _ in 0..1_000_000 {
            self.poll_control(state);
            if state.port_ready {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    fn initialize(&self, state: &mut State) {
        self.set_status(0);
        let initial_base = *self.base.lock();
        let mut device_base = None;
        for slot in 0..8 {
            let base = initial_base + slot * 0x1000;
            if Self::read_at(base, REG_MAGIC) != MAGIC {
                continue;
            }
            let version = Self::read_at(base, REG_VERSION);
            if version == VERSION_MODERN && Self::read_at(base, REG_DEVICE_ID) == DEVICE_CONSOLE {
                device_base = Some(base);
                break;
            }
        }
        let Some(device_base) = device_base else {
            self.fail();
            return;
        };
        *self.base.lock() = device_base;

        self.set_status(STATUS_ACKNOWLEDGE);
        self.set_status(STATUS_ACKNOWLEDGE | STATUS_DRIVER);

        self.write(REG_DEVICE_FEATURES_SEL, 0);
        let feature_low = self.read(REG_DEVICE_FEATURES) as u64;
        self.write(REG_DEVICE_FEATURES_SEL, 1);
        let feature_high = self.read(REG_DEVICE_FEATURES) as u64;
        let device_features = feature_low | (feature_high << 32);
        if device_features & FEATURE_VERSION_1 == 0 {
            self.fail();
            return;
        }

        self.write(REG_DRIVER_FEATURES_SEL, 0);
        self.write(REG_DRIVER_FEATURES, 0);
        self.write(REG_DRIVER_FEATURES_SEL, 1);
        self.write(REG_DRIVER_FEATURES, (FEATURE_VERSION_1 >> 32) as u32);

        self.set_status(STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK);
        if self.read(REG_STATUS) & STATUS_FEATURES_OK == 0 {
            self.fail();
            return;
        }

        let Some(tx_size) = self.configure_queue(QUEUE_TX, &mut state.tx) else {
            self.fail();
            return;
        };
        let Some(rx_size) = self.configure_queue(QUEUE_RX, &mut state.rx) else {
            self.fail();
            return;
        };
        let Some(ctrl_rx_size) = self.configure_queue(QUEUE_CTRL_RX, &mut state.ctrl_rx) else {
            self.fail();
            return;
        };
        let Some(ctrl_tx_size) = self.configure_queue(QUEUE_CTRL_TX, &mut state.ctrl_tx) else {
            self.fail();
            return;
        };
        state.tx_size = tx_size;
        state.rx_size = rx_size;
        state.ctrl_rx_size = ctrl_rx_size;
        state.ctrl_tx_size = ctrl_tx_size;
        state.tx_last_used = 0;
        state.rx_last_used = 0;
        state.ctrl_tx_last_used = 0;
        state.ctrl_rx_last_used = 0;
        state.rx_length = 0;
        state.rx_offset = 0;
        state.port_ready = false;
        self.post_receive_buffer(state);
        self.post_control_receive_buffer(state);
        self.set_status(STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK);
        state.ready = true;
        self.send_control(state, 0, CTRL_DEVICE_READY, 1);
    }
}

impl SerialPort for VirtioConsole {
    fn init(&self) {
        let mut state = self.state.lock();
        if !state.ready {
            self.initialize(&mut state);
        }
    }

    fn putc(&self, byte: u8) {
        let mut state = self.state.lock();
        if !state.ready {
            return;
        }
        if !self.ensure_port_ready(&mut state) {
            return;
        }

        state.tx.buffer[0] = byte;
        let buffer_address = state.tx.buffer.as_ptr() as usize as u64;
        let descriptor = &mut state.tx.descriptors[0];
        descriptor.address = buffer_address;
        descriptor.length = 1;
        descriptor.flags = 0;
        descriptor.next = 0;

        let slot = (state.tx.available.index as usize) % state.tx_size as usize;
        unsafe {
            write_volatile(&mut state.tx.available.ring[slot], 0);
            fence(Ordering::SeqCst);
            write_volatile(
                &mut state.tx.available.index,
                state.tx.available.index.wrapping_add(1),
            );
        }
        self.write(REG_QUEUE_NOTIFY, QUEUE_TX);

        while unsafe { read_volatile(&state.tx.used.index) } == state.tx_last_used {
            core::hint::spin_loop();
        }
        state.tx_last_used = unsafe { read_volatile(&state.tx.used.index) };
        self.ack_interrupt();
    }

    fn try_getc(&self) -> Option<u8> {
        let mut state = self.state.lock();
        if !state.ready {
            return None;
        }
        self.poll_control(&mut state);

        if state.rx_offset < state.rx_length {
            let byte = state.rx.buffer[state.rx_offset];
            state.rx_offset += 1;
            return Some(byte);
        }

        if state.rx_length != 0 {
            state.rx_length = 0;
            state.rx_offset = 0;
            self.post_receive_buffer(&mut state);
        }

        let used_index = unsafe { read_volatile(&state.rx.used.index) };
        if used_index == state.rx_last_used {
            self.ack_interrupt();
            return None;
        }

        let slot = (state.rx_last_used as usize) % state.rx_size as usize;
        let length = unsafe { read_volatile(&state.rx.used.ring[slot].length) } as usize;
        state.rx_last_used = state.rx_last_used.wrapping_add(1);
        state.rx_length = length.min(BUFFER_SIZE);
        state.rx_offset = 0;
        self.ack_interrupt();

        if state.rx_length == 0 {
            None
        } else {
            let byte = state.rx.buffer[0];
            state.rx_offset = 1;
            Some(byte)
        }
    }

    fn enable_rx_interrupt(&self) {}

    fn disable_rx_interrupt(&self) {}
}
