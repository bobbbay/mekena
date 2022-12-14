use tokio::select;

use crate::{context::Context, node::Node};

pub struct System {
    state: SystemState,
    nodes: Vec<Box<dyn Node + 'static>>, // TODO: can we figure this out at compile time?
    context: Context,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum SystemState {
    #[default]
    NotStarted,
    Starting,
    Running,
    Stopping,
}

pub enum NextState {
    Continue,
    Stop,
}

impl System {
    pub fn new() -> Self {
        Self {
            state: SystemState::default(),
            nodes: Vec::new(),
            context: Context::new(),
        }
    }

    /// Register a node.
    pub fn add_node(mut self, node: impl Node + 'static) -> Self {
        self.nodes.push(Box::new(node));
        self
    }

    pub async fn start(&mut self) -> Result<(), SystemError> {
        match self.starting().await? {
            NextState::Continue => match self.running().await? {
                _ => self.stopping().await?,
            },
            NextState::Stop => self.stopping().await?,
        };

        Ok(())
    }

    async fn starting(&mut self) -> Result<NextState, SystemError> {
        self.state = SystemState::Starting;

        let output = futures::future::join_all(
            self.nodes
                .iter_mut()
                .map(|x: &mut Box<dyn Node + 'static>| x.starting(&self.context)),
        );

        select! {
            _ = output => Ok(NextState::Continue),
            _ = self.context.await_shutdown() => Ok(NextState::Stop),
        }
    }

    async fn running(&mut self) -> Result<NextState, SystemError> {
        self.state = SystemState::Running;

        let output = futures::future::join_all(
            self.nodes
                .iter_mut()
                .map(|x: &mut Box<dyn Node + 'static>| x.running(&self.context)),
        );

        select! {
            _ = output => Ok(NextState::Continue),
            _ = self.context.await_shutdown() => Ok(NextState::Stop),
        }
    }

    async fn stopping(&mut self) -> Result<NextState, SystemError> {
        self.state = SystemState::Stopping;

        let output = futures::future::join_all(
            self.nodes
                .iter_mut()
                .map(|x: &mut Box<dyn Node + 'static>| x.stopping(&self.context)),
        );

        select! {
            _ = output => Ok(NextState::Continue),
            _ = self.context.await_shutdown() => Ok(NextState::Stop),
        }
    }

    pub fn get_state(&self) -> SystemState {
        self.state
    }
}

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
pub enum SystemError {
    #[error("The context was commanded to shut down.")]
    #[diagnostic(code(mekena::system::shutdown))]
    Shutdown,

    #[error("An unknown error occured.")]
    #[diagnostic(code(mekena::system::unknown))]
    Unknown,
}
