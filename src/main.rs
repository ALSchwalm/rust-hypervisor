#![no_std]
#![no_main]
#![feature(asm)]
#![feature(never_type)]
#![feature(const_fn)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate log;

#[no_mangle]
pub static _fltused: u32 = 0;

use uefi::prelude::*;

mod device;
mod efialloc;
mod error;
mod memory;
mod percpu;
mod registers;
mod vm;
#[allow(dead_code)]
mod vmcs;
mod vmexit;
mod vmx;

#[entry]
fn efi_main(_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&system_table).expect_success("Failed to initialize utilities");

    let mut alloc = efialloc::EfiAllocator::new(system_table.boot_services());

    let mut vmx = vmx::Vmx::enable(&mut alloc).expect("Failed to enable vmx");

    let mut config = vm::VirtualMachineConfig::new(1024);

    // FIXME: When `load_image` may return an error, log the error.
    //
    // Map OVMF directly below the 4GB boundary
    config
        .load_image(
            "OVMF.fd".into(),
            memory::GuestPhysAddr::new((4 * 1024 * 1024 * 1024) - (2 * 1024 * 1024)),
        )
        .unwrap_or(());
    config.register_device(device::ComDevice::new(0x3F8));
    config.register_device(device::ComDevice::new(0x402)); // The qemu debug port
    config.register_device(device::PciRootComplex::new());

    let vm = vm::VirtualMachine::new(&mut vmx, &mut alloc, config, system_table.boot_services())
        .expect("Failed to create vm");

    info!("Constructed VM!");

    vm.launch(vmx).expect("Failed to launch vm");
}
