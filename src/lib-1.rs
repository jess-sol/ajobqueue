use std::any::Any;

use linkme::distributed_slice;

pub trait Job {

}

pub trait JobType<D: Any> {
    fn job_type(&self) -> String;
    fn job_data(&self) -> D;
}

pub struct CommonJobType {}

impl JobType<String> for CommonJobType {
    fn job_type(&self) -> String {
        String::from("default")
    }

    fn job_data(&self) -> String {
        String::from("Hello, World!")
    }
}

// === TODO - Make macro for registration of job types
#[distributed_slice(JOB_TYPES)]
fn make_common_job() -> Box<dyn JobType<dyn Any>> {
    Box::new(CommonJobType {}) as _
}
// ===

#[distributed_slice]
pub static JOB_TYPES: [fn() -> Box<dyn JobType<dyn Any>>] = [..];


#[cfg(test)]
mod tests {
    use crate::JOB_TYPES;
    #[test]
    fn test_works() {
        // let job_types = JOB_TYPES.iter().map(|con| con().job_type()).collect::<Vec<_>>();
        // println!("HELLO: {}", job_types.join(", "));
    }
}
