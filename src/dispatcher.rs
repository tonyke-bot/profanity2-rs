use ocl::{
    ffi::{cl_event, cl_ulong},
    prm::Ulong4,
    Context, Device, Event, EventList, Program,
};
use std::{
    ffi::c_void,
    sync::{Mutex, RwLock},
};

use crate::{
    clear_p, clear_pln, compute_unit::ComputeUnit, config::Config, types::HashResult, utils,
};

pub struct CallbackParam<'a> {
    pub dispatcher: *mut Dispatcher<'a>,
    pub compute_unit: *mut ComputeUnit<'a>,
}

pub struct Dispatcher<'a> {
    context: Context,
    program: Program,
    config: &'a Config,
    self_ptr: *mut Dispatcher<'a>,

    time_start: std::time::Instant,
    max_score: Mutex<usize>,
    total_size: usize,
    total_initialized: RwLock<usize>,
    finish_events: Vec<Event>,
    compute_units: Vec<ComputeUnit<'a>>,
}

impl<'a> Dispatcher<'a> {
    pub fn new(context: Context, program: Program, config: &'a Config) -> Self {
        let mut dispatcher = Self {
            context,
            program,
            total_initialized: RwLock::new(0),
            total_size: 0,
            self_ptr: std::ptr::null_mut(),
            time_start: std::time::Instant::now(),

            finish_events: vec![],
            compute_units: vec![],
            max_score: Mutex::new(0),

            config,
        };

        dispatcher.self_ptr = &mut dispatcher as *mut Dispatcher;

        dispatcher
    }

    pub fn add_device(&mut self, device: Device, device_index: usize) {
        let event = Event::user(&self.context).unwrap();
        let cu = ComputeUnit::new(
            self.context.clone(),
            self.program.clone(),
            device_index,
            device,
            &self.config,
            event.clone(),
        );

        self.total_size += self.config.max_work_size;
        self.compute_units.push(cu);
        self.finish_events.push(event);
    }

    extern "C" fn init_callback(_: cl_event, _: i32, callback_param: *mut c_void) {
        let param = unsafe { Box::from_raw(callback_param as *mut CallbackParam) };
        let dispatcher = unsafe { &mut *param.dispatcher };
        let compute_unit = unsafe { &mut *param.compute_unit };

        let total_initialized = {
            let mut guard = dispatcher.total_initialized.write().unwrap();
            *guard += compute_unit.get_last_init_size();
            *guard
        };

        clear_p!(
            "  {:.2}%",
            total_initialized as f64 * 100f64 / dispatcher.total_size as f64
        );

        let done = compute_unit.init_continue(
            Dispatcher::init_callback,
            Box::into_raw(param) as *mut c_void,
        );

        if done {
            clear_pln!("  GPU{} initialized", compute_unit.get_device_index());
        }
    }

    pub fn init(&mut self) {
        for cu in &mut self.compute_units {
            cu.start_init();

            let boxed_param_ptr = Box::into_raw(Box::new(CallbackParam {
                dispatcher: self.self_ptr,
                compute_unit: cu as *mut ComputeUnit,
            })) as *mut c_void;

            cu.init_continue(Dispatcher::init_callback, boxed_param_ptr);
        }

        EventList::from(self.finish_events.clone())
            .wait_for()
            .unwrap();
    }

    extern "C" fn compute_callback(_: cl_event, _: i32, callback_param: *mut c_void) {
        let param = unsafe { Box::from_raw(callback_param as *mut CallbackParam) };
        let dispatcher = unsafe { &mut *param.dispatcher };
        let compute_unit = unsafe { &mut *param.compute_unit };

        compute_unit.increase_round();
        dispatcher.handle_result(&compute_unit, compute_unit.get_last_result());
        dispatcher.print_speed();
        compute_unit.compute_continue(
            true,
            Dispatcher::compute_callback,
            Box::into_raw(param) as *mut c_void,
        );
    }

    pub fn run(&mut self) {
        self.time_start = std::time::Instant::now();
        let event_finished = Event::user(&self.context).unwrap();

        for cu in &mut self.compute_units {
            let boxed_param_ptr = Box::into_raw(Box::new(CallbackParam {
                dispatcher: self.self_ptr,
                compute_unit: cu as *mut ComputeUnit,
            })) as *mut c_void;

            cu.compute(Dispatcher::compute_callback, boxed_param_ptr);
        }

        event_finished.wait_for().unwrap();
    }

    fn handle_result(&mut self, cu: &ComputeUnit, result: &Vec<HashResult>) {
        let mut max_score = { *self.max_score.lock().unwrap() };
        let mut i = result.len() - 1;

        while i > max_score {
            let r = &result[i];
            let new_max_score = i as u8;

            if r.found > 0 && (new_max_score) > cu.get_max_score() {
                cu.set_max_score(new_max_score);

                {
                    let mut guard = self.max_score.lock().unwrap();
                    if i > *guard {
                        max_score = i;
                        *guard = i;
                    }
                }

                self.print_result(i, cu.get_round(), cu.get_seed(), r);
                // TODO: quit when goal achieved.
            }

            i -= 1;
        }
    }

    fn print_result(&self, score: usize, round: usize, seed: Ulong4, result: &HashResult) {
        let time_elapsed = self.time_start.elapsed().as_millis();
        let round = round as cl_ulong;

        let seed0 = seed[0] + round;
        let seed1 = seed[1] + if seed0 < round { 1 } else { 0 };
        let seed2 = seed[2] + if seed1 == 0 { 1 } else { 0 };
        let seed3 = seed[3] + if seed2 == 0 { 1 } else { 0 } + result.found_id as cl_ulong;

        let address = utils::to_checksum_address(&result.found_hash);
        let time = if time_elapsed < 1000 {
            format!("{}ms", time_elapsed)
        } else {
            format!("{}s", time_elapsed / 1000)
        };

        clear_pln!(
            "  Score: {:<2} Time: {:<7} {}: 0x{} Key: {:016x}{:016x}{:016x}{:016x}",
            score,
            time,
            self.config.target,
            address,
            seed3,
            seed2,
            seed1,
            seed0
        );
    }

    fn print_speed(&self) {
        let mut total_speed = 0f64;
        let mut message = String::with_capacity(512);

        if self.config.compact_speed {
            total_speed = self
                .compute_units
                .iter()
                .map(|cu| cu.get_speed())
                .sum::<f64>();
        } else {
            message.push_str(" -");

            for (i, cu) in self.compute_units.iter().enumerate() {
                let speed = cu.get_speed();
                total_speed += speed;

                message.push_str(&format!(" GPU {}: {}", i, format_speed(speed)));
            }
        }

        clear_p!("Total Speed: {:>10}{}", format_speed(total_speed), message);
    }
}

fn format_speed(speed: f64) -> String {
    let speed = speed / 1024f64 / 1024f64;

    if speed < 1024f64 {
        format!("{:.2}MB/s", speed)
    } else {
        format!("{:.2}GB/s", speed / 1024f64)
    }
}
