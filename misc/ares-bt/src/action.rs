use crate::{
    Behavior, CancelSafe, EternalBehavior, EternalStatus, FallibleBehavior, FallibleStatus,
    InfallibleBehavior, InfallibleStatus, IntoRon, Status,
};

impl<T, F: FnMut(&mut B) -> Status<T>, B> Behavior<B, T> for F {
    fn run(&mut self, blackboard: &mut B) -> Status<T> {
        self(blackboard)
    }
}

impl<T, F: FnMut(&mut B) -> InfallibleStatus<T>, B> InfallibleBehavior<B, T> for F {
    fn run_infallible(&mut self, blackboard: &mut B) -> InfallibleStatus<T> {
        self(blackboard)
    }
}

impl<T, F: FnMut(&mut B) -> FallibleStatus<T>, B> FallibleBehavior<B, T> for F {
    fn run_fallible(&mut self, blackboard: &mut B) -> FallibleStatus<T> {
        self(blackboard)
    }
}

impl<T, F: FnMut(&mut B) -> EternalStatus<T>, B> EternalBehavior<B, T> for F {
    fn run_eternal(&mut self, blackboard: &mut B) -> EternalStatus<T> {
        self(blackboard)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AlwaysSucceed;

impl<T, B> Behavior<B, T> for AlwaysSucceed {
    fn run(&mut self, _blackboard: &mut B) -> Status<T> {
        Status::Success
    }
}

impl<T, B> InfallibleBehavior<B, T> for AlwaysSucceed {
    fn run_infallible(&mut self, _blackboard: &mut B) -> InfallibleStatus<T> {
        InfallibleStatus::Success
    }
}

impl IntoRon for AlwaysSucceed {
    fn into_ron(&self) -> ron::Value {
        ron::Value::String("AlwaysSucceed".to_string())
    }
}

impl CancelSafe for AlwaysSucceed {
    fn reset(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AlwaysFail;

impl<T, B> Behavior<B, T> for AlwaysFail {
    fn run(&mut self, _blackboard: &mut B) -> Status<T> {
        Status::Failure
    }
}

impl<T, B> FallibleBehavior<B, T> for AlwaysFail {
    fn run_fallible(&mut self, _blackboard: &mut B) -> FallibleStatus<T> {
        FallibleStatus::Failure
    }
}

impl IntoRon for AlwaysFail {
    fn into_ron(&self) -> ron::Value {
        ron::Value::String("AlwaysFail".to_string())
    }
}

impl CancelSafe for AlwaysFail {
    fn reset(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AlwaysRunning;

impl<T: Default, B> Behavior<B, T> for AlwaysRunning {
    fn run(&mut self, _blackboard: &mut B) -> Status<T> {
        Status::Running(Default::default())
    }
}

impl<T: Default, B> InfallibleBehavior<B, T> for AlwaysRunning {
    fn run_infallible(&mut self, _blackboard: &mut B) -> InfallibleStatus<T> {
        InfallibleStatus::Running(Default::default())
    }
}

impl<T: Default, B> FallibleBehavior<B, T> for AlwaysRunning {
    fn run_fallible(&mut self, _blackboard: &mut B) -> FallibleStatus<T> {
        FallibleStatus::Running(Default::default())
    }
}

impl<T: Default, B> EternalBehavior<B, T> for AlwaysRunning {
    fn run_eternal(&mut self, _blackboard: &mut B) -> EternalStatus<T> {
        Default::default()
    }
}

impl CancelSafe for AlwaysRunning {
    fn reset(&mut self) {}
}

impl IntoRon for AlwaysRunning {
    fn into_ron(&self) -> ron::Value {
        ron::Value::String("AlwaysRunning".to_string())
    }
}

pub struct RunOnce<F> {
    pub func: F,
    ran: bool,
}

impl<F> From<F> for RunOnce<F> {
    fn from(func: F) -> Self {
        Self { func, ran: false }
    }
}

impl<B, T, F: FnMut() -> T> Behavior<B, T> for RunOnce<F> {
    fn run(&mut self, _blackboard: &mut B) -> Status<T> {
        if self.ran {
            self.ran = false;
            Status::Success
        } else {
            self.ran = true;
            Status::Running((self.func)())
        }
    }
}

impl<B, T, F: FnMut() -> T> InfallibleBehavior<B, T> for RunOnce<F> {
    fn run_infallible(&mut self, _blackboard: &mut B) -> InfallibleStatus<T> {
        if self.ran {
            self.ran = false;
            InfallibleStatus::Success
        } else {
            self.ran = true;
            InfallibleStatus::Running((self.func)())
        }
    }
}

impl<F> CancelSafe for RunOnce<F> {
    fn reset(&mut self) {
        self.ran = false;
    }
}
