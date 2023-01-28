use thiserror::Error;
use smithay_client_toolkit::reexports::{
    calloop::Error as CalloopError,
    client::{
        globals::GlobalError,
        ConnectError,
    },
};

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    Connect(#[from] ConnectError),
    #[error(transparent)]
    Global(#[from] GlobalError),
    #[error(transparent)]
    Calloop(#[from] CalloopError),
}

pub type AppResult<T> = Result<T, AppError>;
