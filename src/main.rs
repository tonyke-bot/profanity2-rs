use ocl::{
    enums::{DeviceInfo, DeviceInfoResult},
    Context, Device, DeviceType, Platform, Program,
};

use crate::dispatcher::Dispatcher;

mod commands;
mod compute_unit;
mod config;
mod dispatcher;
#[macro_use]
mod macros;
mod precomp;
mod speed_meter;
mod types;
mod utils;

fn main() {
    let cli = commands::parse_cli();
    let config = cli.to_config();

    let platforms = Platform::list();
    let devices = Device::list(platforms[0], Some(DeviceType::GPU)).unwrap();

    pln!("Mode: {}", config.mode);
    pln!("Target: {}", config.target);

    let mut available_devices = 0;
    let skip_devices = cli.get_skip_devices();

    pln!("Devices: ");
    for (i, device) in devices.iter().enumerate() {
        if skip_devices.contains(&i) {
            continue;
        }

        let DeviceInfoResult::MaxComputeUnits(max_compute_units) = device.info(DeviceInfo::MaxComputeUnits).unwrap() else { panic!(); };
        let DeviceInfoResult::GlobalMemSize(global_mem_size) = device.info(DeviceInfo::GlobalMemSize).unwrap() else { panic!(); };

        pln!(
            "  GPU {index}: {name}, {mem_size:.2} GB available, {max_compute_units} compute units",
            index = i,
            name = device.name().unwrap(),
            mem_size = global_mem_size as f64 / 1024.0 / 1024.0 / 1024.0,
            max_compute_units = max_compute_units,
        );

        available_devices += 1;
    }

    if available_devices == 0 {
        pln!("  No available devices found!");
        return;
    } else {
        pln!();
    }

    pln!("Initializing OpenCL...");
    p!("  Creating context...");

    let context = Context::builder()
        .platform(platforms[0])
        .devices(&devices)
        .build()
        .unwrap();

    pln!("OK");

    p!("  Compiling kernels...");

    let program = Program::builder()
        .source_file("keccak.cl")
        .source_file("profanity.cl")
        .cmplr_opt(format!("-D PROFANITY_INVERSE_SIZE={}", config.inverse_size))
        .cmplr_opt(format!("-D PROFANITY_MAX_SCORE={}", config.max_score))
        .devices(&devices)
        .build(&context)
        .unwrap();
    pln!("OK");

    // TODO: cached built program
    let mut dispatcher = Dispatcher::new(context, program, &config);

    pln!("");
    pln!("Initializing devices...");
    pln!("  This should take less than a minute. The number of objects initialized on each");
    pln!("  device is equal to inverse-size * inverse-multiple. To lower initialization");
    pln!("  time (and memory footprint) I suggest lowering the inverse-multiple first.");
    pln!("  You can do this via the -I switch. Do note that this might negatively impact");
    pln!("  your performance.");
    pln!();

    for (i, device) in devices.iter().enumerate() {
        if skip_devices.contains(&i) {
            continue;
        }

        dispatcher.add_device(*device, i);
    }
    dispatcher.init();
    pln!();

    pln!("Running...");
    pln!("  Always verify that a private key generated by this program corresponds to the");
    pln!("  public key printed by importing it to a wallet of your choice. This program");
    pln!("  like any software might contain bugs and it does by design cut corners to");
    pln!("  improve overall performance.");
    pln!();
    dispatcher.run();
}
