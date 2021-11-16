use super::fiber::Fiber;

pub struct Vm {
    children: Vec<Runnable>,
}

struct Child {
    priority: f64,
    runnable: Runnable,
}

enum Runnable {
    Fiber(Fiber),
    Vm(Vm),
}

impl Vm {
    pub fn new() {}
}
