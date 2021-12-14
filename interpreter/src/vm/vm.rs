use std::collections::HashMap;

use crate::compiler::byte_code::ByteCode;

use super::fiber::Fiber;
use super::{utils::*, FiberStatus};
use itertools::Itertools;
use log::debug;
use rand::prelude::*;

#[derive(Debug)]
pub struct Vm {
    status: VmStatus,
    children: Vec<Child>,
    external_pending_operations: Vec<VmOperation>,
    internal_pending_operations: Vec<VmOperation>,
    external_to_internal_channel: HashMap<ChannelId, ChannelId>,
    next_channel_id: ChannelId,
    channels: HashMap<ChannelId, ExternalOrInternalChannel>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VmStatus {
    Running,
    Done(Value),
    Panicked(Value),
    WaitingForPendingOperations,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum VmOperation {
    Send(ChannelId, Value),
    Receive(ChannelId),
}

#[derive(Debug)]
struct Child {
    priority: f64,
    runnable: Runnable,
}
#[derive(Debug)]
enum Runnable {
    Fiber(Fiber),
    Vm(Vm),
}
impl Runnable {
    pub fn run(&mut self, num_instructions: u16) {
        match self {
            Runnable::Fiber(fiber) => fiber.run(num_instructions),
            Runnable::Vm(vm) => vm.run(num_instructions),
        }
    }
}

#[derive(Debug)]
enum ExternalOrInternalChannel {
    External(ChannelId),
    Internal(Channel),
}
impl ExternalOrInternalChannel {
    fn to_internal(&mut self) -> &mut Channel {
        match self {
            ExternalOrInternalChannel::External(_) => panic!(),
            ExternalOrInternalChannel::Internal(channel) => channel,
        }
    }
}

#[derive(Debug)]
struct Channel {
    capacity: u64,
    messages: Vec<Value>,
}
impl Channel {
    fn send(&mut self, message: Value) -> bool {
        if self.messages.len() < self.capacity as usize {
            self.messages.push(message);
            true
        } else {
            false
        }
    }
    fn receive(&mut self) -> Option<Value> {
        if self.messages.len() > 0 {
            Some(self.messages.remove(0))
        } else {
            None
        }
    }
}

impl Vm {
    pub fn new(byte_code: ByteCode, ambients: HashMap<String, Value>) -> Self {
        let mut vm = Self {
            status: VmStatus::Running,
            children: vec![],
            external_pending_operations: vec![],
            internal_pending_operations: vec![],
            external_to_internal_channel: HashMap::new(),
            next_channel_id: 0,
            channels: HashMap::new(),
        };
        let main_fiber = Fiber::new(
            byte_code,
            ambients
                .into_iter()
                .map(|(key, value)| (key, vm.import(value)))
                .collect(),
            Value::unit(),
        );
        vm.children.push(Child {
            priority: 1.0,
            runnable: Runnable::Fiber(main_fiber),
        });
        vm
    }
    fn import(&mut self, value: Value) -> Value {
        match value {
            Value::Int(int) => Value::Int(int),
            Value::String(string) => Value::String(string),
            Value::Symbol(symbol) => Value::Symbol(symbol),
            Value::Map(map) => Value::Map(
                map.into_iter()
                    .map(|(key, value)| (self.import(key), self.import(value)))
                    .collect(),
            ),
            Value::List(list) => Value::List(list.into_iter().map(|it| self.import(it)).collect()),
            Value::Closure { captured, body } => Value::Closure {
                captured: captured.into_iter().map(|it| self.import(it)).collect(),
                body,
            },
            Value::ChannelSendEnd(id) => {
                Value::ChannelSendEnd(self.internal_channel_for_external(id))
            }
            Value::ChannelReceiveEnd(id) => {
                Value::ChannelReceiveEnd(self.internal_channel_for_external(id))
            }
        }
    }
    fn internal_channel_for_external(&mut self, external_id: ChannelId) -> ChannelId {
        match self.external_to_internal_channel.get(&external_id) {
            Some(id) => *id,
            None => {
                let internal_id = self.next_channel_id;
                self.external_to_internal_channel
                    .insert(external_id, internal_id);
                self.channels.insert(
                    internal_id,
                    ExternalOrInternalChannel::External(external_id),
                );
                self.next_channel_id += 1;
                internal_id
            }
        }
    }

    pub fn status(&self) -> VmStatus {
        self.status.clone()
    }
    pub fn pending_operations(&self) -> &[VmOperation] {
        &self.external_pending_operations
    }

    pub fn run(&mut self, num_instructions: u16) {
        let priority_sums = self
            .children
            .iter()
            .filter(|it| match &it.runnable {
                Runnable::Fiber(fiber) => fiber.status() == FiberStatus::Running,
                Runnable::Vm(vm) => vm.status() == VmStatus::Running,
            })
            .scan(0.0, |acc, x| {
                *acc = *acc + x.priority;
                Some((x, *acc))
            })
            .collect_vec();
        if priority_sums.is_empty() {
            self.status = VmStatus::WaitingForPendingOperations;
            return;
        }
        let chosen = random::<f64>().rem_euclid(priority_sums.last().unwrap().1);
        let chosen = priority_sums.iter().position(|it| chosen < it.1).unwrap();
        debug!("VM: {:?}", self);
        let chosen = self.children.get_mut(chosen).unwrap();
        chosen.runnable.run(num_instructions);
        match &mut chosen.runnable {
            Runnable::Fiber(fiber) => match fiber.status() {
                FiberStatus::Running => {}
                FiberStatus::Done(value) => self.status = VmStatus::Done(value),
                FiberStatus::Panicked(value) => self.status = VmStatus::Panicked(value),
                FiberStatus::CreatingChannel(capacity) => {
                    let id = self.next_channel_id;
                    self.next_channel_id += 1;
                    self.channels.insert(
                        id,
                        ExternalOrInternalChannel::Internal(Channel {
                            capacity,
                            messages: vec![],
                        }),
                    );
                    fiber.resolve_creating_channel(id);
                }
                FiberStatus::Sending(channel, message) => {
                    debug!("Channels: {:?}", self.channels);
                    let (queue, id) = match self.channels.get_mut(&channel).unwrap() {
                        ExternalOrInternalChannel::External(id) => {
                            (&mut self.external_pending_operations, *id)
                        }
                        ExternalOrInternalChannel::Internal(_) => {
                            (&mut self.internal_pending_operations, channel)
                        }
                    };
                    queue.push(VmOperation::Send(id, message));
                }
                FiberStatus::Receiving(channel) => {
                    let (queue, id) = match self.channels.get_mut(&channel).unwrap() {
                        ExternalOrInternalChannel::External(id) => {
                            (&mut self.external_pending_operations, *id)
                        }
                        ExternalOrInternalChannel::Internal(_) => {
                            (&mut self.internal_pending_operations, channel)
                        }
                    };
                    queue.push(VmOperation::Receive(id));
                }
            },
            Runnable::Vm(vm) => match vm.status() {
                VmStatus::Running | VmStatus::WaitingForPendingOperations => {}
                VmStatus::Done(value) => println!("VM finished with value {}.", value),
                VmStatus::Panicked(value) => println!("VM panicked with value {}.", value),
            },
        }

        // Go through the internal pending operations as long as we can serve
        // some of them.
        let mut last_round = self.internal_pending_operations.len() + 1;
        while last_round > self.internal_pending_operations.len() {
            last_round = self.internal_pending_operations.len();

            let mut index = 0;
            while index < self.internal_pending_operations.len() {
                match self.internal_pending_operations.get(index).unwrap() {
                    VmOperation::Send(id, message) => {
                        let channel = self.channels.get_mut(&id).unwrap().to_internal();
                        if channel.send(message.clone()) {
                            'find_send_consumer: for child in &mut self.children {
                                match &mut child.runnable {
                                    Runnable::Fiber(fiber) => match fiber.status() {
                                        FiberStatus::Sending(the_id, the_message) => {
                                            if the_id == *id && &the_message == message {
                                                fiber.resolve_sending();
                                                break 'find_send_consumer;
                                            }
                                        }
                                        _ => {}
                                    },
                                    Runnable::Vm(vm) => {
                                        for operation in vm.pending_operations() {
                                            if let VmOperation::Send(the_id, the_message) =
                                                operation
                                            {
                                                if the_id == id && the_message == message {
                                                    vm.resolve_send(*id, message.clone());
                                                    break 'find_send_consumer;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            index += 1;
                            continue;
                        }
                    }
                    VmOperation::Receive(id) => {
                        let channel = self.channels.get_mut(&id).unwrap().to_internal();
                        if let Some(message) = channel.receive() {
                            'find_receive_consumer: for child in &mut self.children {
                                match &mut child.runnable {
                                    Runnable::Fiber(fiber) => match fiber.status() {
                                        FiberStatus::Receiving(the_id) => {
                                            if the_id == *id {
                                                fiber.resolve_receiving(message);
                                                break 'find_receive_consumer;
                                            }
                                        }
                                        _ => {}
                                    },
                                    Runnable::Vm(vm) => {
                                        for operation in vm.pending_operations() {
                                            if let VmOperation::Receive(the_id) = operation {
                                                if the_id == id {
                                                    vm.resolve_receive(*id, message.clone());
                                                    break 'find_receive_consumer;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            index += 1;
                            continue;
                        }
                    }
                }
                self.internal_pending_operations.remove(index);
            }
        }
    }

    pub fn resolve_send(&mut self, channel: ChannelId, message: Value) {
        let operation = VmOperation::Send(channel, message.clone());
        let index = self
            .external_pending_operations
            .iter()
            .position(|it| *it == operation)
            .unwrap();
        self.external_pending_operations.remove(index);
        let internal_id = self.external_to_internal_channel[&channel];
        let resolve_candidates = self.children.iter_mut().filter(|it| match &it.runnable {
            Runnable::Fiber(fiber) => {
                fiber.status() == FiberStatus::Sending(internal_id, message.clone())
            }
            Runnable::Vm(vm) => {
                let operation = VmOperation::Send(internal_id, message.clone());
                vm.external_pending_operations.contains(&operation)
            }
        });
        let resolved = resolve_candidates
            .into_iter()
            .choose(&mut thread_rng())
            .unwrap();
        match &mut resolved.runnable {
            Runnable::Fiber(fiber) => fiber.resolve_sending(),
            Runnable::Vm(vm) => vm.resolve_send(channel, message),
        }
        self.status = VmStatus::Running;
    }

    pub fn resolve_receive(&mut self, channel: ChannelId, message: Value) {
        let operation = VmOperation::Receive(channel);
        let index = self
            .external_pending_operations
            .iter()
            .position(|it| *it == operation)
            .unwrap();
        self.external_pending_operations.remove(index);
        let internal_id = self.external_to_internal_channel[&channel];
        let resolve_candidates = self.children.iter_mut().filter(|it| match &it.runnable {
            Runnable::Fiber(fiber) => fiber.status() == FiberStatus::Receiving(internal_id),
            Runnable::Vm(vm) => vm
                .external_pending_operations
                .contains(&VmOperation::Receive(internal_id)),
        });
        let resolved = resolve_candidates
            .into_iter()
            .choose(&mut thread_rng())
            .unwrap();
        match &mut resolved.runnable {
            Runnable::Fiber(fiber) => fiber.resolve_receiving(message),
            Runnable::Vm(vm) => vm.resolve_receive(channel, message),
        }
        self.status = VmStatus::Running;
    }
}
