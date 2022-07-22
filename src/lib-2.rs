use linkme::distributed_slice;

pub enum Pipe { Stdout, Stderr }

pub trait Job {
    fn run(&self);
}

pub trait PrintJobType: Job {
    fn data(&self) -> Pipe;
}

struct PrintJob {
    msg: String,
}

impl PrintJobType for PrintJob {
    fn data(&self) -> Pipe {
        Pipe::Stdout
    }
}

impl Job for PrintJob {
    fn run(&self) {
        match self.data() {
            Pipe::Stdout => println!("{}", self.msg),
            Pipe::Stderr => eprintln!("Error: {}", self.msg),
        }
    }
}

// === TODO - Make macro for registration of job types
#[distributed_slice(JOB_TYPES)]
fn make_print_job() -> Box<dyn Job> {
    Box::new(PrintJob { msg: String::from("Hello, World") }) as _
}
// ===

#[distributed_slice]
pub static JOB_TYPES: [fn() -> Box<dyn Job>] = [..];

#[cfg(test)]
mod tests {
    use crate::JOB_TYPES;

    #[test]
    fn it_works() {
        let job = JOB_TYPES.first().unwrap()();
        job.run();
    }
}
