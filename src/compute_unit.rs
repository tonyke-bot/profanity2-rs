use ocl::{
    core::{Error as OclCoreError, EventCallbackFn, Status},
    prm::{cl_uchar, cl_ulong, Ulong4},
    Buffer, Context, Device, Error, Event, Kernel, MemFlags, Program, Queue, SpatialDims,
};
use secp256k1::rand::{rngs::OsRng, RngCore};
use std::{cmp::min, ffi::c_void, sync::Mutex};

use crate::{
    clear_pln,
    config::{Config, HashTarget},
    precomp::PRECOMP_DATA,
    speed_meter::SpeedMeter,
    types::{HashResult, MpNumber, Point},
    types::{ScoreData, SCORE_DATA_SIZE},
};

pub struct ComputeUnit<'a> {
    config: &'a Config,
    device_index: usize,
    program: Program,
    queue: Queue,
    size: usize,
    finish_event: Event,

    mem_precomp: Option<Buffer<Point>>,
    mem_result: Buffer<HashResult>,
    mem_points_delta_x: Buffer<MpNumber>,
    mem_inversed_negative_double_gy: Buffer<MpNumber>,
    mem_prev_lambda: Buffer<MpNumber>,

    seed: Ulong4,
    seed_x: Ulong4,
    seed_y: Ulong4,
    max_score: Mutex<cl_ulong>,

    round: usize,
    size_initialized: usize,
    last_init_size: usize,
    last_result: Vec<HashResult>,

    speed_meter: SpeedMeter,

    kernel_inverse: Option<Kernel>,
    kernel_iterate: Option<Kernel>,
    kernel_contract_transform: Option<Kernel>,
    kernel_score: Option<Kernel>,
}

impl<'a> ComputeUnit<'a> {
    pub fn new(
        context: Context,
        program: Program,
        device_index: usize,
        device: Device,
        config: &'a Config,
        finish_event: Event,
    ) -> Self {
        let size = config.max_work_size;
        let queue = Queue::new(&context, device, None).unwrap();

        let mem_result = Buffer::builder()
            .queue(queue.clone())
            .flags(MemFlags::READ_WRITE | MemFlags::HOST_READ_ONLY)
            .len(config.get_max_score() + 1)
            .build()
            .unwrap();

        let mem_points_delta_x = Buffer::builder()
            .queue(queue.clone())
            .flags(MemFlags::READ_WRITE | MemFlags::HOST_NO_ACCESS)
            .len(size)
            .build()
            .unwrap();
        let mem_inversed_negative_double_gy = Buffer::builder()
            .queue(queue.clone())
            .flags(MemFlags::READ_WRITE | MemFlags::HOST_NO_ACCESS)
            .len(size)
            .build()
            .unwrap();
        let mem_prev_lambda = Buffer::builder()
            .queue(queue.clone())
            .flags(MemFlags::READ_WRITE | MemFlags::HOST_NO_ACCESS)
            .len(size)
            .build()
            .unwrap();

        ComputeUnit {
            program,
            size,
            finish_event,
            device_index,
            queue: queue.clone(),
            speed_meter: SpeedMeter::new(config.get_speed_meter_sample_count()),

            size_initialized: 0,
            last_init_size: 0,
            last_result: Vec::new(),

            seed: create_seed(),
            seed_x: Ulong4::new(
                cl_ulong::from_be_bytes(config.public_key.as_slice()[24..32].try_into().unwrap()),
                cl_ulong::from_be_bytes(config.public_key.as_slice()[16..24].try_into().unwrap()),
                cl_ulong::from_be_bytes(config.public_key.as_slice()[8..16].try_into().unwrap()),
                cl_ulong::from_be_bytes(config.public_key.as_slice()[0..8].try_into().unwrap()),
            ),
            seed_y: Ulong4::new(
                cl_ulong::from_be_bytes(config.public_key.as_slice()[56..64].try_into().unwrap()),
                cl_ulong::from_be_bytes(config.public_key.as_slice()[48..56].try_into().unwrap()),
                cl_ulong::from_be_bytes(config.public_key.as_slice()[40..48].try_into().unwrap()),
                cl_ulong::from_be_bytes(config.public_key.as_slice()[32..40].try_into().unwrap()),
            ),
            round: 0,
            max_score: Mutex::new(0),

            mem_precomp: None,
            mem_result,
            mem_points_delta_x,
            mem_inversed_negative_double_gy,
            mem_prev_lambda,

            kernel_inverse: None,
            kernel_iterate: None,
            kernel_contract_transform: None,
            kernel_score: None,

            config,
        }
    }

    pub fn start_init(&mut self) {
        let mem_precomp = unsafe {
            Buffer::builder()
                .queue(self.queue.clone())
                .flags(MemFlags::new().read_only())
                .use_host_slice(PRECOMP_DATA.as_slice())
                .len(PRECOMP_DATA.len())
                .build()
                .unwrap()
        };

        self.last_init_size = 0;
        self.round = 0;
        self.mem_precomp = Some(mem_precomp);
    }

    pub fn init_continue(
        &mut self,
        callback_receiver: EventCallbackFn,
        callback_param: *mut c_void,
    ) -> bool {
        let init_batch_size = self.size / 50;
        self.size_initialized += self.last_init_size;
        let finished = self.size_initialized >= self.size;

        if !finished {
            let size_to_work = min(self.size - self.size_initialized, init_batch_size);
            let kernel_init = Kernel::builder()
                .program(&self.program)
                .queue(self.queue.clone())
                .name("profanity_init")
                .global_work_size(SpatialDims::One(size_to_work))
                .global_work_offset(SpatialDims::One(self.size_initialized))
                .arg_named("precomp", self.mem_precomp.as_ref().unwrap())
                .arg_named("pDeltaX", &self.mem_points_delta_x)
                .arg_named("pPrevLambda", &self.mem_prev_lambda)
                .arg_named("pResult", &self.mem_result)
                .arg_named("seed", self.seed)
                .arg_named("seedX", self.seed_x)
                .arg_named("seedY", self.seed_y)
                .build()
                .unwrap();

            let mut event = Event::empty();
            unsafe { kernel_init.cmd().enew(&mut event).enq().unwrap() };
            self.queue.flush().unwrap();

            self.last_init_size = size_to_work;

            unsafe { event.set_callback(callback_receiver, callback_param) }.unwrap();
        } else {
            self.mem_precomp = None;
            self.finish_event.set_complete().unwrap();
        }

        finished
    }

    pub fn compute(&mut self, callback_receiver: EventCallbackFn, callback_param: *mut c_void) {
        self.kernel_inverse = Some(
            Kernel::builder()
                .program(&self.program)
                .queue(self.queue.clone())
                .name("profanity_inverse")
                .arg_named("pDeltaX", &self.mem_points_delta_x)
                .arg_named("pInverse", &self.mem_inversed_negative_double_gy)
                .global_work_size(SpatialDims::One(self.config.inverse_multiplier))
                .local_work_size(SpatialDims::One(self.config.local_work_size))
                .build()
                .unwrap(),
        );

        self.kernel_iterate = Some(
            Kernel::builder()
                .program(&self.program)
                .queue(self.queue.clone())
                .name("profanity_iterate")
                .arg_named("pDeltaX", &self.mem_points_delta_x)
                .arg_named("pInverse", &self.mem_inversed_negative_double_gy)
                .arg_named("pPrevLambda", &self.mem_prev_lambda)
                .global_work_size(SpatialDims::One(self.size))
                .local_work_size(SpatialDims::One(self.config.local_work_size))
                .build()
                .unwrap(),
        );

        if let HashTarget::Contract = self.config.target {
            self.kernel_contract_transform = Some(
                Kernel::builder()
                    .program(&self.program)
                    .queue(self.queue.clone())
                    .name("profanity_transform_contract")
                    .arg_named("pInverse", &self.mem_inversed_negative_double_gy)
                    .global_work_size(SpatialDims::One(self.size))
                    .local_work_size(SpatialDims::One(self.config.local_work_size))
                    .build()
                    .unwrap(),
            );
        }

        let (data1, data2) = self.config.mode.get_data();
        let (buffer1, buffer2) = (
            build_score_data_buffer(&self.queue, &data1.unwrap_or_default()),
            build_score_data_buffer(&self.queue, &data2.unwrap_or_default()),
        );

        self.kernel_score = Some(
            Kernel::builder()
                .program(&self.program)
                .queue(self.queue.clone())
                .name(self.config.mode.get_kernel_name())
                .arg_named("pInverse", &self.mem_inversed_negative_double_gy)
                .arg_named("pResult", &self.mem_result)
                .arg_named("data1", buffer1)
                .arg_named("data2", buffer2)
                .arg_named("scoreMax", 0)
                .global_work_size(SpatialDims::One(self.size))
                .local_work_size(SpatialDims::One(self.config.local_work_size))
                .build()
                .unwrap(),
        );

        self.speed_meter.reset();
        self.compute_continue(false, callback_receiver, callback_param);
    }

    pub fn compute_continue(
        &mut self,
        from_completion: bool,
        callback_receiver: EventCallbackFn,
        callback_param: *mut c_void,
    ) {
        if from_completion {
            self.speed_meter.log(self.size);
        }

        let mut event = Event::empty();

        self.last_result = vec![HashResult::new(); self.config.get_max_score() + 1];
        self.mem_result
            .read(&mut self.last_result)
            .enew(&mut event)
            .enq()
            .unwrap();

        enqueue_kernel(self.device_index, self.kernel_inverse.as_mut().unwrap());
        enqueue_kernel(self.device_index, self.kernel_iterate.as_mut().unwrap());

        if let Some(kernel_contract_transform) = self.kernel_contract_transform.as_mut() {
            enqueue_kernel(self.device_index, kernel_contract_transform);
        }

        let mut kernel_score = self.kernel_score.as_mut().unwrap();
        kernel_score
            .set_arg("scoreMax", *self.max_score.lock().unwrap())
            .unwrap();

        enqueue_kernel(self.device_index, &mut kernel_score);

        self.queue.flush().unwrap();

        unsafe { event.set_callback(callback_receiver, callback_param) }.unwrap();
    }

    pub fn get_last_init_size(&self) -> usize {
        self.last_init_size
    }

    pub fn get_last_result(&self) -> &Vec<HashResult> {
        &self.last_result
    }

    pub fn set_max_score(&self, max_score: cl_ulong) {
        *self.max_score.lock().unwrap() = max_score;
    }

    pub fn get_max_score(&self) -> cl_ulong {
        *self.max_score.lock().unwrap()
    }

    pub fn get_seed(&self) -> Ulong4 {
        self.seed
    }

    pub fn increase_round(&mut self) {
        self.round += 1;
    }

    pub fn get_round(&self) -> usize {
        self.round
    }

    pub fn get_speed(&self) -> f64 {
        self.speed_meter.get_speed()
    }

    pub fn get_device_index(&self) -> usize {
        self.device_index
    }
}

fn enqueue_kernel(device_index: usize, kernel: &mut Kernel) {
    unsafe { kernel.enq() }
        .or_else(|e| {
            if let Error::OclCore(OclCoreError::Api(api_error)) = &e {
                let status = api_error.status();
                let local_work_size = match kernel.default_local_work_size() {
                    SpatialDims::One(size) => size,
                    _ => 0,
                };

                if (status == Status::CL_INVALID_WORK_GROUP_SIZE
                    || status == Status::CL_INVALID_WORK_ITEM_SIZE)
                    && local_work_size != 0
                {
                    clear_pln!(
                        "  Warning: local work size {} abandoned on GPU {}",
                        local_work_size,
                        device_index
                    );
                    kernel.set_default_local_work_size(SpatialDims::One(0));
                    unsafe { kernel.enq() }.unwrap()
                }
            }

            Err(e)
        })
        .unwrap();
}

fn create_seed() -> Ulong4 {
    let mut rng = OsRng;

    Ulong4::new(
        rng.next_u64(),
        rng.next_u64(),
        rng.next_u64(),
        rng.next_u64(),
    )
}

fn build_score_data_buffer(queue: &Queue, data: &ScoreData) -> Buffer<cl_uchar> {
    let buffer = Buffer::builder()
        .queue(queue.clone())
        .flags(MemFlags::new().read_only().copy_host_ptr())
        .len(SCORE_DATA_SIZE)
        .copy_host_slice(data)
        .build()
        .unwrap();

    return buffer;
}
